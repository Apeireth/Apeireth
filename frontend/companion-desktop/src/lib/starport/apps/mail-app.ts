// 信使 · 邮件台装配层（批次推进）
//
// 契约：
//   1. 装配层薄 —— 行为在 mail-store.ts（测试钉死），本文件只做 DOM 装配；
//   2. 真实文件面 —— 导入走 <input type=file> 真读文件、导出走 Blob 真下载，
//      往返经 parseEml/toEml 同一进出闸；
//   3. 传输协议面（SMTP/IMAP）是诚实接线缝——界面明示待组装，不设假发送钮；
//   4. mount 返回 cleanup（清理恰好一次）。
//
// 源镜像钉接线见 tests/starport-face-apps.mjs 系（随集成面入册）。

import type {AppContribution} from '../contribution-registry';
import {
  toEml,
  parseEml,
  transitionStatus,
  createMailbox,
  type EmailMessage,
} from './mail-store.ts';

interface SmtpSendReceipt {
  accepted: string[];
  server_response: string;
  duration_ms: number;
}

interface ImapEnvelope {
  uid: number;
  from: string;
  to: string;
  subject: string;
  date: string;
  size_bytes: number;
}

interface ImapFetchResult {
  folder: string;
  uid_validity: number;
  exists: number;
  envelopes: ImapEnvelope[];
  truncated: boolean;
  duration_ms: number;
}

function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const {invoke} = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

type Doc = Document;

function el<K extends keyof HTMLElementTagNameMap>(
  doc: Doc,
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = doc.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function createMailApp(): AppContribution {
  return {
    id: 'starport-mail',
    title: '信使',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const mailbox = createMailbox();
      const note = el(doc, 'div', 'mail-note', '本地邮件台：.eml 导入/导出走真实文件面；SMTP 发送走真实传输（隐式 TLS / 明文双确认；STARTTLS 与 IMAP 收信为接线缝）；密码只随本次发送走内存，不落存储。');
      const form = el(doc, 'div', 'mail-form');
      const fromInput = el(doc, 'input', 'mail-from');
      fromInput.placeholder = 'From: me@home.test';
      const toInput = el(doc, 'input', 'mail-to');
      toInput.placeholder = 'To: you@there.test（多个逗号分隔）';
      const subjectInput = el(doc, 'input', 'mail-subject');
      subjectInput.placeholder = '主题';
      const bodyInput = el(doc, 'textarea', 'mail-body');
      bodyInput.placeholder = '正文';
      const saveBtn = el(doc, 'button', 'mail-save', '存草稿');
      form.append(fromInput, toInput, subjectInput, bodyInput, saveBtn);

      const toolRow = el(doc, 'div', 'mail-tools');
      const importInput = el(doc, 'input', 'mail-import');
      importInput.type = 'file';
      importInput.accept = '.eml,text/plain';
      toolRow.append(importInput);

      const smtpRow = el(doc, 'div', 'mail-smtp');
      const hostInput = el(doc, 'input', 'mail-host');
      hostInput.placeholder = 'SMTP 主机';
      const portInput = el(doc, 'input', 'mail-port');
      portInput.placeholder = '端口(465)';
      const tlsInput = el(doc, 'select', 'mail-tls');
      for (const mode of ['implicit', 'plain-insecure']) {
        const option = el(doc, 'option', undefined, mode);
        option.value = mode;
        tlsInput.append(option);
      }
      const userInput = el(doc, 'input', 'mail-user');
      userInput.placeholder = '用户名（可空）';
      const passInput = el(doc, 'input', 'mail-pass');
      passInput.type = 'password';
      passInput.placeholder = '密码（只走内存）';
      smtpRow.append(hostInput, portInput, tlsInput, userInput, passInput);

      const imapRow = el(doc, 'div', 'mail-imap');
      const imapHost = el(doc, 'input', 'mail-imap-host');
      imapHost.placeholder = 'IMAP 主机';
      const imapPort = el(doc, 'input', 'mail-imap-port');
      imapPort.placeholder = '端口(993)';
      const imapTls = el(doc, 'select', 'mail-imap-tls');
      for (const mode of ['implicit', 'starttls', 'plain-insecure']) {
        const option = el(doc, 'option', undefined, mode);
        option.value = mode;
        imapTls.append(option);
      }
      const imapUser = el(doc, 'input', 'mail-imap-user');
      imapUser.placeholder = '收信用户';
      const imapPass = el(doc, 'input', 'mail-imap-pass');
      imapPass.type = 'password';
      imapPass.placeholder = '收信口令（只走内存）';
      const imapFolder = el(doc, 'input', 'mail-imap-folder');
      imapFolder.value = 'INBOX';
      imapFolder.placeholder = '文件夹';
      const fetchBtn = el(doc, 'button', 'mail-fetch', '收信');
      imapRow.append(imapHost, imapPort, imapTls, imapUser, imapPass, imapFolder, fetchBtn);

      const listHost = el(doc, 'div', 'mail-list');
      const imapPane = el(doc, 'div', 'mail-imap-pane');
      root.append(note, form, toolRow, smtpRow, imapRow, listHost, imapPane);

      async function sendMessage(message: EmailMessage): Promise<void> {
        if (!isDesktop()) {
          note.textContent = 'SMTP 发送需要桌面壳（当前为 Web 构建）——不设假发送。';
          return;
        }
        note.textContent = '发送中…';
        try {
          const receipt = await tauriInvoke<SmtpSendReceipt>('send_smtp', {
            config: {
              host: hostInput.value.trim(),
              port: Number(portInput.value.trim() || '465'),
              tls: tlsInput.value,
              allow_insecure_plain: tlsInput.value === 'plain-insecure',
              username: userInput.value.trim() || null,
              password: passInput.value || null,
              from: message.from,
            },
            eml: toEml(message),
            recipients: message.to,
          });
          const sent = transitionStatus(message, 'sent');
          mailbox.remove(message.id);
          mailbox.add(sent);
          note.textContent = `已投递（${receipt.duration_ms}ms）：${receipt.server_response}`;
          render();
        } catch (caught) {
          note.textContent = `发送失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      }

      async function fetchMail(): Promise<void> {
        if (!isDesktop()) {
          note.textContent = 'IMAP 收信需要桌面壳（当前为 Web 构建）——不设假收信。';
          return;
        }
        note.textContent = '收信中…';
        try {
          const result = await tauriInvoke<ImapFetchResult>('imap_fetch', {
            config: {
              host: imapHost.value.trim(),
              port: Number(imapPort.value.trim() || '993'),
              tls: imapTls.value,
              allowInsecurePlain: imapTls.value === 'plain-insecure',
              user: imapUser.value.trim(),
              password: imapPass.value,
            },
            folder: imapFolder.value.trim() || 'INBOX',
            limit: 20,
          });
          imapPane.textContent = '';
          imapPane.append(
            el(doc, 'div', 'mail-imap-stat', `${result.folder} · 共 ${result.exists} 封 · 取回 ${result.envelopes.length} 封 · ${result.duration_ms}ms${result.truncated ? ' · 已截断' : ''}`),
          );
          for (const envelope of result.envelopes) {
            imapPane.append(
              el(doc, 'div', 'mail-imap-row', `#${envelope.uid} · ${envelope.subject || '（无主题）'} · ${envelope.from} · ${envelope.date} · ${envelope.size_bytes}B`),
            );
          }
          note.textContent = `收信完成：${result.envelopes.length} 封信封（整封正文取回为增强位，本批信封级收信）。`;
        } catch (caught) {
          note.textContent = `收信失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      }

      function render(): void {
        listHost.textContent = '';
        for (const message of mailbox.list()) {
          const row = el(doc, 'div', 'mail-row');
          row.append(
            el(doc, 'span', 'mail-chip', `${message.direction === 'in' ? '收' : '发'}·${message.status}`),
            el(doc, 'span', 'mail-subject-view', message.subject || '（无主题）'),
          );
          const exportBtn = el(doc, 'button', 'mail-export', '导出 .eml');
          exportBtn.addEventListener('click', () => {
            const blob = new Blob([toEml(message)], {type: 'message/rfc822'});
            const url = URL.createObjectURL(blob);
            const link = el(doc, 'a');
            link.href = url;
            link.download = `${message.id}.eml`;
            link.click();
            URL.revokeObjectURL(url);
            note.textContent = `已导出：${message.id}.eml`;
          });
          row.append(exportBtn);
          if (message.status === 'draft') {
            const queueBtn = el(doc, 'button', 'mail-queue', '入队');
            queueBtn.addEventListener('click', () => {
              try {
                const next = transitionStatus(message, 'queued');
                mailbox.remove(message.id);
                mailbox.add(next);
                render();
              } catch (caught) {
                note.textContent = `操作失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
              }
            });
            row.append(queueBtn);
          }
          if (message.status === 'queued') {
            const sendBtn = el(doc, 'button', 'mail-send', '发送');
            sendBtn.addEventListener('click', () => void sendMessage(message));
            row.append(sendBtn);
          }
          listHost.append(row);
        }
      }

      saveBtn.addEventListener('click', () => {
        try {
          mailbox.add({
            id: `mail-${Date.now()}`,
            direction: 'out',
            from: fromInput.value,
            to: toInput.value.split(',').map((entry) => entry.trim()).filter(Boolean),
            subject: subjectInput.value,
            body: bodyInput.value,
            at: Date.now(),
            status: 'draft',
          } as EmailMessage);
          note.textContent = '草稿已存（本地账本）。';
          render();
        } catch (caught) {
          note.textContent = `存稿失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      });

      fetchBtn.addEventListener('click', () => void fetchMail());
      importInput.addEventListener('change', () => {
        const file = importInput.files?.[0];
        if (!file) return;
        void file
          .text()
          .then((raw) => {
            const message = parseEml(raw, `mail-${Date.now()}`);
            mailbox.add(message);
            note.textContent = `已导入：${file.name}`;
            render();
          })
          .catch((caught: unknown) => {
            note.textContent = `导入失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
          });
      });

      render();
      return () => {
        root.textContent = '';
      };
    },
  };
}
