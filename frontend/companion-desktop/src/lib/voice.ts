// Apeireth 2.0 Real-time Voice Duplex & Audio Visualizer Engine

export interface VoiceState {
  isCalling: boolean;
  isMuted: boolean;
  isUserSpeaking: boolean;
  isAssistantSpeaking: boolean;
  statusText: string;
  transcript: string;
  audioVolume: number;
}

export class VoiceCallManager {
  private audioCtx: AudioContext | null = null;
  private mediaStream: MediaStream | null = null;
  private analyser: AnalyserNode | null = null;
  private dataArray: Uint8Array | null = null;
  private isRunning: boolean = false;
  private isMuted: boolean = false;
  private recognition: any = null;

  public state: VoiceState = {
    isCalling: false,
    isMuted: false,
    isUserSpeaking: false,
    isAssistantSpeaking: false,
    statusText: '就绪',
    transcript: '',
    audioVolume: 0,
  };

  private stateListeners: Array<(s: VoiceState) => void> = [];
  private onUserSpeechEndCallback: ((text: string) => void) | null = null;

  public subscribe(fn: (s: VoiceState) => void): () => void {
    this.stateListeners.push(fn);
    fn({ ...this.state });
    return () => {
      this.stateListeners = this.stateListeners.filter((l) => l !== fn);
    };
  }

  private notify() {
    const copy = { ...this.state };
    for (const l of this.stateListeners) {
      l(copy);
    }
  }

  public setOnUserSpeechEnd(cb: (text: string) => void) {
    this.onUserSpeechEndCallback = cb;
  }

  public async startCall(): Promise<boolean> {
    try {
      this.audioCtx = new (window.AudioContext || (window as any).webkitAudioContext)();
      this.mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });

      const source = this.audioCtx.createMediaStreamSource(this.mediaStream);
      this.analyser = this.audioCtx.createAnalyser();
      this.analyser.fftSize = 256;
      source.connect(this.analyser);

      const bufferLength = this.analyser.frequencyBinCount;
      this.dataArray = new Uint8Array(bufferLength);

      this.isRunning = true;
      this.state.isCalling = true;
      this.state.statusText = '正在聆听...';
      this.notify();

      this.startAudioLoop();
      this.startSpeechRecognition();
      return true;
    } catch (e) {
      console.error('Failed to start voice call:', e);
      this.state.statusText = '麦克风权限受限或音频初始化失败';
      this.notify();
      return false;
    }
  }

  private startAudioLoop() {
    const tick = () => {
      if (!this.isRunning || !this.analyser || !this.dataArray) return;

      this.analyser.getByteFrequencyData(this.dataArray);
      let sum = 0;
      for (let i = 0; i < this.dataArray.length; i++) {
        sum += this.dataArray[i];
      }
      const avg = sum / this.dataArray.length;
      this.state.audioVolume = avg / 255;

      const isSpeaking = this.state.audioVolume > 0.08 && !this.isMuted;
      if (isSpeaking !== this.state.isUserSpeaking) {
        this.state.isUserSpeaking = isSpeaking;
        if (isSpeaking && this.state.isAssistantSpeaking) {
          // Local Barge-In: interrupt TTS if user speaks
          this.interruptAssistantSpeech();
        }
        this.notify();
      }

      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }

  private startSpeechRecognition() {
    const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (!SpeechRecognition) {
      this.state.transcript = '(浏览器不支持原生 Web Speech API，使用文本模式)';
      this.notify();
      return;
    }

    this.recognition = new SpeechRecognition();
    this.recognition.continuous = true;
    this.recognition.interimResults = true;
    this.recognition.lang = 'zh-CN';

    this.recognition.onresult = (event: any) => {
      let interim = '';
      let final = '';
      for (let i = event.resultIndex; i < event.results.length; ++i) {
        if (event.results[i].isFinal) {
          final += event.results[i][0].transcript;
        } else {
          interim += event.results[i][0].transcript;
        }
      }

      this.state.transcript = final || interim;
      this.notify();

      if (final.trim().length > 0 && this.onUserSpeechEndCallback) {
        this.onUserSpeechEndCallback(final.trim());
      }
    };

    this.recognition.onerror = (err: any) => {
      console.warn('Speech recognition error:', err);
    };

    try {
      this.recognition.start();
    } catch (e) {
      console.warn('Recognition already started:', e);
    }
  }

  public speak(text: string, onEnd?: () => void) {
    if (!window.speechSynthesis) return;

    this.interruptAssistantSpeech();

    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = 'zh-CN';
    utterance.rate = 1.05;
    utterance.pitch = 1.0;

    utterance.onstart = () => {
      this.state.isAssistantSpeaking = true;
      this.state.statusText = 'Apeireth 正在回答...';
      this.notify();
    };

    utterance.onend = () => {
      this.state.isAssistantSpeaking = false;
      this.state.statusText = '正在聆听...';
      this.notify();
      if (onEnd) onEnd();
    };

    utterance.onerror = () => {
      this.state.isAssistantSpeaking = false;
      this.state.statusText = '正在聆听...';
      this.notify();
    };

    window.speechSynthesis.speak(utterance);
  }

  public interruptAssistantSpeech() {
    if (window.speechSynthesis) {
      window.speechSynthesis.cancel();
    }
    this.state.isAssistantSpeaking = false;
    this.notify();
  }

  public toggleMute() {
    this.isMuted = !this.isMuted;
    this.state.isMuted = this.isMuted;
    this.notify();
  }

  public endCall() {
    this.isRunning = false;
    this.interruptAssistantSpeech();

    if (this.recognition) {
      try { this.recognition.stop(); } catch {}
      this.recognition = null;
    }

    if (this.mediaStream) {
      this.mediaStream.getTracks().forEach((t) => t.stop());
      this.mediaStream = null;
    }

    if (this.audioCtx) {
      this.audioCtx.close();
      this.audioCtx = null;
    }

    this.state.isCalling = false;
    this.state.isUserSpeaking = false;
    this.state.isAssistantSpeaking = false;
    this.state.statusText = '通话已结束';
    this.notify();
  }
}

export const voiceCallManager = new VoiceCallManager();
