// 星语 · 语音面装配（faces.ts 星语 → 真实工作切片）
//
// 契约（I-starport-builtins.md「星语」）：
//   1. 真实硬件 I/O —— getUserMedia({audio:true}) 取真麦克风；AudioContext +
//      AnalyserNode 取真实时域样本算电平（电平数学复用 voice-store）；
//      MediaRecorder 真实录制；<audio> 真实回放；
//   2. 绝不假波形 —— 电平条宽度只来自 AnalyserNode 真数据经 rmsLevel +
//      smoothLevel 归约，无增益美化、无合成动画；峰值读数走 peakHold；
//   3. 如实外显 —— 权限拒绝 / 设备缺失 / 能力未组装一律外显错误帧原文，
//      不吞错、不粉饰、不降级装好；
//   4. 清理恰好一次 —— 停轨道 + 关音频上下文 + 中止录制 + 回收全部对象
//      URL + 摘监听 + 清根节点；
//   5. 合成真件 —— window.speechSynthesis 系统语音合成器真合成（WebView2
//      走系统语音引擎，真声非假按钮）；能力未支持如实禁用+说明。转写引擎
//      仍为诚实接线缝（界面注明，不设假按钮）。
//
// 状态迁移全部走 voice-store.transitionVoiceState（合法表之外抛错，事件
// 处理器兜底外显）。DOM 面收窄：createElement/textContent/value/hidden/
// addEventListener + ownerDocument，零 innerHTML（防注入纪律延伸到渲染面）。

import type {AppContribution} from '../contribution-registry';
import {
  transitionVoiceState,
  rmsLevel,
  smoothLevel,
  peakHold,
  VoiceLedger,
  VoiceStoreError,
  formatDuration,
  transitionSpeechState,
  buildSpeechUtterance,
  SpeechJobLedger,
  transitionTranscriptState,
  TranscriptLedger,
  type VoiceState,
  type VoiceAction,
  type SpeechState,
  type SpeechAction,
  type TranscriptState,
  type TranscriptAction,
  type TranscriptRecord,
} from './voice-store.ts';

export const STARPORT_VOICE_APP_ID = 'starport-voice';

const STATE_LABELS: Readonly<Record<VoiceState, string>> = {
  idle: '待命',
  requesting: '申请麦克风…',
  recording: '录音中',
  stopped: '已停止',
  error: '出错',
};

const SPEECH_LABELS: Readonly<Record<SpeechState, string>> = {
  idle: '待命',
  speaking: '朗读中',
  done: '已读完',
  error: '出错',
};

const TRANSCRIPT_LABELS: Readonly<Record<TranscriptState, string>> = {
  idle: '待命',
  recognizing: '聆听中…',
  done: '已定稿',
  error: '出错',
};

/** 桌面壳守卫与 Tauri 命令调用（terminal-app 同款成例；真件调用无假回退）。 */
function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const {invoke} = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

/** 电平平滑系数（显示用指数平滑；数据源恒为真实样本）。 */
const LEVEL_SMOOTH_FACTOR = 0.3;
/** 峰值保持窗（帧数）。 */
const PEAK_WINDOW = 32;

function messageOf(caught: unknown): string {
  return caught instanceof Error ? caught.message : String(caught);
}

function isDenied(caught: unknown): boolean {
  return (
    typeof DOMException !== 'undefined' &&
    caught instanceof DOMException &&
    (caught.name === 'NotAllowedError' ||
      caught.name === 'PermissionDeniedError' ||
      caught.name === 'SecurityError')
  );
}

export function createVoiceApp(): AppContribution {
  return {
    id: STARPORT_VOICE_APP_ID,
    title: '星语',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc: Document = root.ownerDocument ?? document;

      const ledger = new VoiceLedger();
      const transcriptLedger = new TranscriptLedger();
      let transcriptState: TranscriptState = 'idle';
      let transcriptSeq = 0;
      let lastTranscript: {text: string; confidence?: number} | null = null;
      let asrAvailable = false;
      const noteUrls = new Map<string, string>();
      let state: VoiceState = 'idle';
      let mediaStream: MediaStream | null = null;
      let audioCtx: AudioContext | null = null;
      let analyser: AnalyserNode | null = null;
      let recorder: MediaRecorder | null = null;
      let levelBuffer: Float32Array<ArrayBuffer> | null = null;
      let pendingChunks: Blob[] = [];
      let rafId = 0;
      let startedAt = 0;
      let smoothed = 0;
      const recentLevels: number[] = [];
      let seq = 0;
      let cancelled = false;
      let disposables: (() => void)[] = [];

      // —— DOM（textContent 上屏，零 innerHTML） ——
      const statusLine = doc.createElement('div');
      statusLine.className = 'starvoice-status';
      const meter = doc.createElement('div');
      meter.className = 'starvoice-meter';
      const levelBar = doc.createElement('div');
      levelBar.className = 'starvoice-level';
      meter.appendChild(levelBar);
      const peakLine = doc.createElement('div');
      peakLine.className = 'starvoice-peak';
      const actions = doc.createElement('div');
      actions.className = 'starvoice-actions';
      const startBtn = doc.createElement('button');
      startBtn.className = 'starvoice-btn';
      startBtn.textContent = '开始录音';
      const stopBtn = doc.createElement('button');
      stopBtn.className = 'starvoice-btn';
      stopBtn.textContent = '停止';
      const resetBtn = doc.createElement('button');
      resetBtn.className = 'starvoice-btn';
      resetBtn.textContent = '复位';
      actions.appendChild(startBtn);
      actions.appendChild(stopBtn);
      actions.appendChild(resetBtn);
      const errorFrame = doc.createElement('div');
      errorFrame.className = 'starvoice-error';
      errorFrame.hidden = true;
      const seam = doc.createElement('div');
      seam.className = 'starvoice-seam';
      seam.textContent = '合成已接系统语音合成器（真合成）；转写引擎待组装（诚实接线缝）。';
      // —— 朗读区（TTS 真合成：文本 + 语速/音高 + 朗读/停止） ——
      const speechHost = doc.createElement('div');
      speechHost.className = 'starvoice-speech';
      const speechTitle = doc.createElement('div');
      speechTitle.className = 'starvoice-speech-title';
      speechTitle.textContent = '朗读（系统语音合成器 · 真合成）';
      const speechText = doc.createElement('textarea');
      speechText.className = 'starvoice-speech-text';
      speechText.placeholder = '要朗读的文本';
      const speechControls = doc.createElement('div');
      speechControls.className = 'starvoice-speech-controls';
      const rateLabel = doc.createElement('span');
      rateLabel.textContent = '语速';
      const speechRate = doc.createElement('input');
      speechRate.className = 'starvoice-speech-rate';
      speechRate.type = 'number';
      speechRate.min = '0.1';
      speechRate.max = '3';
      speechRate.step = '0.1';
      speechRate.value = '1';
      const pitchLabel = doc.createElement('span');
      pitchLabel.textContent = '音高';
      const speechPitch = doc.createElement('input');
      speechPitch.className = 'starvoice-speech-pitch';
      speechPitch.type = 'number';
      speechPitch.min = '0.1';
      speechPitch.max = '3';
      speechPitch.step = '0.1';
      speechPitch.value = '1';
      const speakBtn = doc.createElement('button');
      speakBtn.className = 'starvoice-btn';
      speakBtn.textContent = '朗读';
      const speechStopBtn = doc.createElement('button');
      speechStopBtn.className = 'starvoice-btn';
      speechStopBtn.textContent = '停止';
      speechControls.appendChild(rateLabel);
      speechControls.appendChild(speechRate);
      speechControls.appendChild(pitchLabel);
      speechControls.appendChild(speechPitch);
      speechControls.appendChild(speakBtn);
      speechControls.appendChild(speechStopBtn);
      const speechStatus = doc.createElement('div');
      speechStatus.className = 'starvoice-speech-status';
      const speechJobsHost = doc.createElement('div');
      speechJobsHost.className = 'starvoice-speech-jobs';
      speechHost.appendChild(speechTitle);
      speechHost.appendChild(speechText);
      speechHost.appendChild(speechControls);
      speechHost.appendChild(speechStatus);
      speechHost.appendChild(speechJobsHost);
      // —— 转写区（真实识别引擎；探测不到即如实禁用，不设假按钮）——
      const transcriptHost = doc.createElement('div');
      transcriptHost.className = 'starvoice-transcript';
      const transcriptTitle = doc.createElement('div');
      transcriptTitle.className = 'starvoice-speech-title';
      transcriptTitle.textContent = '转写（真实识别引擎）';
      const transcriptControls = doc.createElement('div');
      transcriptControls.className = 'starvoice-speech-controls';
      const transcribeBtn = doc.createElement('button');
      transcribeBtn.className = 'starvoice-btn';
      transcribeBtn.textContent = '开始聆听（10s）';
      transcribeBtn.disabled = true;
      const saveTranscriptBtn = doc.createElement('button');
      saveTranscriptBtn.className = 'starvoice-btn';
      saveTranscriptBtn.textContent = '存入';
      saveTranscriptBtn.disabled = true;
      transcriptControls.appendChild(transcribeBtn);
      transcriptControls.appendChild(saveTranscriptBtn);
      const transcriptStatus = doc.createElement('div');
      transcriptStatus.className = 'starvoice-speech-status';
      transcriptStatus.textContent = '引擎探测中…';
      const transcriptResult = doc.createElement('pre');
      transcriptResult.className = 'starvoice-transcript-result';
      const transcriptsHost = doc.createElement('div');
      transcriptsHost.className = 'starvoice-transcript-list';
      transcriptHost.appendChild(transcriptTitle);
      transcriptHost.appendChild(transcriptControls);
      transcriptHost.appendChild(transcriptStatus);
      transcriptHost.appendChild(transcriptResult);
      transcriptHost.appendChild(transcriptsHost);
      const notesHost = doc.createElement('div');
      notesHost.className = 'starvoice-notes';
      root.appendChild(statusLine);
      root.appendChild(meter);
      root.appendChild(peakLine);
      root.appendChild(actions);
      root.appendChild(errorFrame);
      root.appendChild(seam);
      root.appendChild(speechHost);
      root.appendChild(transcriptHost);
      root.appendChild(notesHost);

      function render(): void {
        statusLine.textContent = `状态：${STATE_LABELS[state]}`;
        startBtn.disabled = !(state === 'idle' || state === 'stopped' || state === 'error');
        stopBtn.disabled = state !== 'recording';
        resetBtn.disabled = !(state === 'stopped' || state === 'error');
      }

      function clearError(): void {
        errorFrame.hidden = true;
        errorFrame.textContent = '';
      }

      function showError(text: string): void {
        errorFrame.hidden = false;
        errorFrame.textContent = text;
      }

      function applyState(action: VoiceAction): void {
        state = transitionVoiceState(state, action);
        render();
      }

      function stopTracks(stream: MediaStream | null): void {
        if (!stream) return;
        for (const track of stream.getTracks()) track.stop();
      }

      function closeAudioGraph(): void {
        if (audioCtx) void audioCtx.close();
        audioCtx = null;
        analyser = null;
        levelBuffer = null;
      }

      function stopMeterLoop(): void {
        if (rafId) cancelAnimationFrame(rafId);
        rafId = 0;
      }

      function meterLoop(): void {
        if (cancelled) return;
        if (analyser && levelBuffer) {
          analyser.getFloatTimeDomainData(levelBuffer);
          smoothed = smoothLevel(smoothed, rmsLevel(levelBuffer), LEVEL_SMOOTH_FACTOR);
          const percent = Math.min(100, Math.max(0, smoothed * 100));
          levelBar.style.width = `${percent.toFixed(1)}%`;
          // 峰值读数走 peakHold：最近 PEAK_WINDOW 帧真实电平的窗口峰值。
          recentLevels.push(smoothed);
          if (recentLevels.length > PEAK_WINDOW) recentLevels.shift();
          const peaks = peakHold(recentLevels, PEAK_WINDOW);
          const windowPeak = peaks.length ? peaks[peaks.length - 1] : smoothed;
          peakLine.textContent = `真实电平 ${(smoothed * 100).toFixed(1)}% · 窗峰值 ${(
            windowPeak * 100
          ).toFixed(1)}%（${PEAK_WINDOW} 帧）`;
        }
        rafId = requestAnimationFrame(meterLoop);
      }

      function renderNotes(): void {
        notesHost.textContent = '';
        const notes = ledger.list().reverse();
        if (!notes.length) {
          const empty = doc.createElement('div');
          empty.className = 'starvoice-empty';
          empty.textContent = '暂无录音';
          notesHost.appendChild(empty);
          return;
        }
        for (const note of notes) {
          const row = doc.createElement('div');
          row.className = 'starvoice-note';
          const label = doc.createElement('span');
          label.className = 'starvoice-note-label';
          label.textContent = `${note.label}（${formatDuration(note.durationMs)}，${note.sizeBytes} 字节）`;
          const audio = doc.createElement('audio');
          audio.controls = true;
          const url = noteUrls.get(note.id);
          if (url) audio.src = url;
          const del = doc.createElement('button');
          del.className = 'starvoice-btn';
          del.textContent = '删除';
          const onDelete = (): void => {
            try {
              ledger.remove(note.id);
              const held = noteUrls.get(note.id);
              if (held) {
                URL.revokeObjectURL(held);
                noteUrls.delete(note.id);
              }
              renderNotes();
            } catch (caught) {
              showError(`删除失败（如实报）：${messageOf(caught)}`);
            }
          };
          del.addEventListener('click', onDelete);
          disposables.push(() => del.removeEventListener('click', onDelete));
          row.appendChild(label);
          row.appendChild(audio);
          row.appendChild(del);
          notesHost.appendChild(row);
        }
      }

      function finalizeRecording(): void {
        stopTracks(mediaStream);
        mediaStream = null;
        closeAudioGraph();
        if (cancelled) return;
        const durationMs = Math.round(performance.now() - startedAt);
        const blob = new Blob(pendingChunks, {
          type: recorder?.mimeType || 'audio/webm',
        });
        pendingChunks = [];
        recorder = null;
        if (durationMs <= 0 || blob.size <= 0) {
          showError('录音过短或为空（如实报），未入账本。');
          return;
        }
        seq += 1;
        // 对象 URL 键与账本 id 同一把（否则删除回收对不上账）。
        const noteId = `vn-${Date.now()}-${seq}`;
        const url = URL.createObjectURL(blob);
        noteUrls.set(noteId, url);
        try {
          ledger.add({
            id: noteId,
            at: Date.now(),
            durationMs,
            sizeBytes: blob.size,
            label: `录音 ${formatDuration(durationMs)}`,
          });
          renderNotes();
        } catch (caught) {
          URL.revokeObjectURL(url);
          noteUrls.delete(noteId);
          showError(`入账失败（如实报）：${messageOf(caught)}`);
        }
      }

      async function startRecording(): Promise<void> {
        if (!(state === 'idle' || state === 'stopped' || state === 'error')) return;
        clearError();
        if (typeof MediaRecorder === 'undefined' || !navigator.mediaDevices?.getUserMedia) {
          showError('当前环境缺录音能力（MediaRecorder / 麦克风采集），如实报。');
          return;
        }
        applyState('request');
        try {
          const stream = await navigator.mediaDevices.getUserMedia({audio: true});
          if (cancelled) {
            stopTracks(stream);
            return;
          }
          mediaStream = stream;
          audioCtx = new AudioContext();
          const source = audioCtx.createMediaStreamSource(stream);
          analyser = audioCtx.createAnalyser();
          analyser.fftSize = 1024;
          source.connect(analyser);
          levelBuffer = new Float32Array(analyser.fftSize);
          pendingChunks = [];
          const rec = new MediaRecorder(stream);
          rec.ondataavailable = (event: BlobEvent) => {
            if (event.data && event.data.size > 0) pendingChunks.push(event.data);
          };
          rec.onstop = () => finalizeRecording();
          rec.onerror = () => {
            if (!cancelled) {
              try {
                applyState('fail');
              } catch {
                // 迁移表外的重复失败帧不二次打扰（错误已在屏）。
              }
              showError('录制运行错误（如实报）。');
            }
          };
          rec.start();
          recorder = rec;
          startedAt = performance.now();
          smoothed = 0;
          applyState('grant');
          meterLoop();
        } catch (caught) {
          stopTracks(mediaStream);
          mediaStream = null;
          closeAudioGraph();
          try {
            applyState(isDenied(caught) ? 'deny' : 'fail');
          } catch {
            // 已在错误态时再失败不重复迁移。
          }
          showError(
            isDenied(caught)
              ? `麦克风权限被拒绝（如实报，不假波形）：${messageOf(caught)}`
              : `麦克风获取失败（如实报）：${messageOf(caught)}`,
          );
        }
      }

      function stopRecording(): void {
        if (state !== 'recording') return;
        stopMeterLoop();
        applyState('stop');
        const rec = recorder;
        if (rec && rec.state !== 'inactive') {
          rec.stop(); // onstop → finalizeRecording（真数据落账本）
        }
      }

      function resetToIdle(): void {
        if (!(state === 'stopped' || state === 'error')) return;
        applyState('reset');
        clearError();
      }

      // —— 合成面（TTS 真合成：系统语音合成器，状态机驱动、事件对账） ——
      let speechState: SpeechState = 'idle';
      const speechLedger = new SpeechJobLedger();
      let speechSeq = 0;
      let currentUtterance: SpeechSynthesisUtterance | null = null;
      const hasTts =
        typeof window !== 'undefined' &&
        'speechSynthesis' in window &&
        typeof SpeechSynthesisUtterance !== 'undefined';

      function renderSpeech(): void {
        if (!hasTts) {
          speechStatus.textContent = '当前环境缺系统语音合成能力（speechSynthesis）——朗读禁用，不设假按钮。';
          speakBtn.disabled = true;
          speechStopBtn.disabled = true;
          return;
        }
        speechStatus.textContent = `合成：${SPEECH_LABELS[speechState]}`;
        speakBtn.disabled = !(speechState === 'idle' || speechState === 'done' || speechState === 'error');
        speechStopBtn.disabled = speechState !== 'speaking';
      }

      function renderSpeechJobs(): void {
        speechJobsHost.textContent = '';
        const jobs = speechLedger.list().reverse();
        if (!jobs.length) {
          const empty = doc.createElement('div');
          empty.className = 'starvoice-speech-empty';
          empty.textContent = '暂无朗读记录';
          speechJobsHost.appendChild(empty);
          return;
        }
        for (const job of jobs) {
          const row = doc.createElement('div');
          row.className = 'starvoice-speech-job';
          const label = doc.createElement('span');
          const snippet = job.text.length > 24 ? `${job.text.slice(0, 24)}…` : job.text;
          label.textContent = snippet;
          const del = doc.createElement('button');
          del.className = 'starvoice-btn';
          del.textContent = '删除';
          const onDelete = (): void => {
            try {
              speechLedger.remove(job.id);
              renderSpeechJobs();
            } catch (caught) {
              showError(`删除失败（如实报）：${messageOf(caught)}`);
            }
          };
          del.addEventListener('click', onDelete);
          disposables.push(() => del.removeEventListener('click', onDelete));
          row.appendChild(label);
          row.appendChild(del);
          speechJobsHost.appendChild(row);
        }
      }

      function applySpeechState(action: SpeechAction): void {
        speechState = transitionSpeechState(speechState, action);
        renderSpeech();
      }

      /** 字段空串 = 缺省 1.0（声明默认值，非修补）；其余必须是有限数值。 */
      function parseSpeechNumber(raw: string): number | undefined {
        return raw.trim() === '' ? undefined : Number(raw);
      }

      function recordSpeechJob(text: string): void {
        speechSeq += 1;
        try {
          speechLedger.add({id: `sj-${Date.now()}-${speechSeq}`, text, at: Date.now()});
          renderSpeechJobs();
        } catch (caught) {
          showError(`语句入账失败（如实报）：${messageOf(caught)}`);
        }
      }

      function speak(): void {
        if (!hasTts || !(speechState === 'idle' || speechState === 'done' || speechState === 'error')) return;
        clearError();
        let params: ReturnType<typeof buildSpeechUtterance>;
        try {
          params = buildSpeechUtterance(speechText.value, {
            rate: parseSpeechNumber(speechRate.value),
            pitch: parseSpeechNumber(speechPitch.value),
          });
        } catch (caught) {
          showError(`朗读参数非法（如实报）：${messageOf(caught)}`);
          return;
        }
        const utterance = new SpeechSynthesisUtterance(params.text);
        utterance.rate = params.rate;
        utterance.pitch = params.pitch;
        utterance.onend = () => {
          // 取消/替换后的迟到事件按 currentUtterance 对账丢弃（不二次迁移）。
          if (cancelled || currentUtterance !== utterance) return;
          currentUtterance = null;
          applySpeechState('finish');
        };
        utterance.onerror = (event: SpeechSynthesisErrorEvent) => {
          if (cancelled || currentUtterance !== utterance) return;
          currentUtterance = null;
          try {
            applySpeechState('fail');
          } catch {
            // 已在错误态不二次迁移。
          }
          showError(`合成失败（如实报）：${event.error}`);
        };
        currentUtterance = utterance;
        applySpeechState('speak');
        recordSpeechJob(params.text);
        window.speechSynthesis.speak(utterance);
      }

      function stopSpeech(): void {
        if (!hasTts || speechState !== 'speaking') return;
        window.speechSynthesis.cancel();
        currentUtterance = null;
        // reset 语义 = 主动停止/取消（回 idle）；迟到事件已按对账丢弃。
        applySpeechState('reset');
      }

      function applyTranscriptState(action: TranscriptAction): void {
        transcriptState = transitionTranscriptState(transcriptState, action);
        renderTranscript();
      }

      function renderTranscript(): void {
        transcriptStatus.textContent = `转写：${TRANSCRIPT_LABELS[transcriptState]}${
          asrAvailable ? ' · 引擎就绪（真实识别）' : ''
        }`;
        transcribeBtn.disabled =
          !asrAvailable || !(transcriptState === 'idle' || transcriptState === 'done' || transcriptState === 'error');
        saveTranscriptBtn.disabled = !(transcriptState === 'done' && lastTranscript?.text);
      }

      function renderTranscripts(): void {
        transcriptsHost.textContent = '';
        const records = transcriptLedger.list().reverse();
        if (records.length === 0) {
          const empty = doc.createElement('div');
          empty.textContent = '暂无入账转写';
          transcriptsHost.appendChild(empty);
          return;
        }
        for (const record of records) {
          const row = doc.createElement('div');
          row.className = 'starvoice-transcript-row';
          row.textContent = `${record.text}${
            record.confidence === undefined ? '' : `（置信 ${record.confidence.toFixed(2)}）`
          }`;
          const del = doc.createElement('button');
          del.className = 'starvoice-btn';
          del.textContent = '删除';
          const onDelete = (): void => {
            try {
              transcriptLedger.remove(record.id);
              renderTranscripts();
            } catch (caught) {
              showError(`删除失败（如实报）：${messageOf(caught)}`);
            }
          };
          del.addEventListener('click', onDelete);
          disposables.push(() => del.removeEventListener('click', onDelete));
          row.appendChild(del);
          transcriptsHost.appendChild(row);
        }
      }

      /** 引擎探测（真探测；不可用即如实禁用，绝不假按钮）。 */
      async function detectEngine(): Promise<void> {
        if (!isDesktop()) {
          transcriptStatus.textContent = '转写需要桌面壳（当前为 Web 构建）——不设假按钮。';
          renderTranscript();
          return;
        }
        try {
          const status = await tauriInvoke<{available: boolean; engine: string; reason: string}>(
            'detect_speech_engine',
          );
          if (cancelled) return;
          asrAvailable = status.available;
          transcriptStatus.textContent = status.available
            ? status.reason
            : `引擎不可用：${status.reason}`;
        } catch (caught) {
          if (cancelled) return;
          asrAvailable = false;
          transcriptStatus.textContent = `引擎探测失败（如实报）：${messageOf(caught)}`;
        }
        renderTranscript();
      }

      /** 真实识别（走 Tauri 真命令；空结果如实，不造文本）。 */
      async function transcribe(): Promise<void> {
        if (!asrAvailable || !(transcriptState === 'idle' || transcriptState === 'done' || transcriptState === 'error')) {
          return;
        }
        clearError();
        lastTranscript = null;
        transcriptResult.textContent = '';
        applyTranscriptState('start');
        try {
          const result = await tauriInvoke<{text: string; confidence: number}>('recognize_speech', {
            durationSecs: 10,
          });
          if (cancelled) return;
          if (result.text.trim()) {
            lastTranscript = {text: result.text.trim(), confidence: result.confidence};
            transcriptResult.textContent = `${result.text.trim()}（置信 ${result.confidence.toFixed(2)}）`;
          } else {
            transcriptResult.textContent = '（未采集到语音——如实空结果，不造文本）';
          }
          applyTranscriptState('finish');
        } catch (caught) {
          if (cancelled) return;
          try {
            applyTranscriptState('fail');
          } catch {
            // 已在错误态不二次迁移。
          }
          showError(`转写失败（如实报）：${messageOf(caught)}`);
        }
        renderTranscript();
      }

      function saveTranscript(): void {
        if (transcriptState !== 'done' || !lastTranscript?.text) return;
        transcriptSeq += 1;
        try {
          transcriptLedger.add({
            id: `tr-${Date.now()}-${transcriptSeq}`,
            at: Date.now(),
            text: lastTranscript.text,
            ...(lastTranscript.confidence === undefined ? {} : {confidence: lastTranscript.confidence}),
          });
          saveTranscriptBtn.disabled = true;
          renderTranscripts();
        } catch (caught) {
          showError(`入账失败（如实报）：${messageOf(caught)}`);
        }
      }

      const onStart = (): void => {
        void startRecording();
      };
      const onStop = (): void => stopRecording();
      const onReset = (): void => resetToIdle();
      startBtn.addEventListener('click', onStart);
      stopBtn.addEventListener('click', onStop);
      resetBtn.addEventListener('click', onReset);
      disposables.push(() => startBtn.removeEventListener('click', onStart));
      disposables.push(() => stopBtn.removeEventListener('click', onStop));
      disposables.push(() => resetBtn.removeEventListener('click', onReset));
      const onSpeak = (): void => speak();
      const onSpeechStop = (): void => stopSpeech();
      speakBtn.addEventListener('click', onSpeak);
      speechStopBtn.addEventListener('click', onSpeechStop);
      disposables.push(() => speakBtn.removeEventListener('click', onSpeak));
      disposables.push(() => speechStopBtn.removeEventListener('click', onSpeechStop));
      const onTranscribe = (): void => {
        void transcribe();
      };
      const onSaveTranscript = (): void => saveTranscript();
      transcribeBtn.addEventListener('click', onTranscribe);
      saveTranscriptBtn.addEventListener('click', onSaveTranscript);
      disposables.push(() => transcribeBtn.removeEventListener('click', onTranscribe));
      disposables.push(() => saveTranscriptBtn.removeEventListener('click', onSaveTranscript));

      render();
      renderNotes();
      renderSpeech();
      renderSpeechJobs();
      renderTranscript();
      renderTranscripts();
      void detectEngine();

      // 清理恰好一次：停轨道 + 关上下文 + 中止录制 + 回收对象 URL + 摘监听。
      return () => {
        if (cancelled) return;
        cancelled = true;
        stopMeterLoop();
        const rec = recorder;
        recorder = null;
        if (rec && rec.state !== 'inactive') {
          rec.onstop = null;
          rec.onerror = null;
          try {
            rec.stop();
          } catch {
            // 已停/已释放不惊扰。
          }
        }
        stopTracks(mediaStream);
        mediaStream = null;
        closeAudioGraph();
        // 合成真停：取消队列朗读（真停非假停），迟到事件按对账丢弃。
        if (hasTts) {
          try {
            window.speechSynthesis.cancel();
          } catch {
            // 引擎已释放不惊扰。
          }
        }
        currentUtterance = null;
        for (const url of noteUrls.values()) URL.revokeObjectURL(url);
        noteUrls.clear();
        for (const dispose of disposables) dispose();
        disposables = [];
        root.textContent = '';
      };
    },
  };
}
