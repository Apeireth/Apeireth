// 舵室 · 终端面装配层（批次推进）
//
// 契约：
//   1. 用户直操面 —— 命令由用户亲自键入执行（工作区钉死/超时强杀/输出截断
//      由原生侧 terminal_exec.rs 承担并测试钉死）；智能体驱动的命令执行必须
//      走审批族（接线缝，届时点亮，绝不绕）；
//   2. 真实执行 —— 走 Tauri run_workspace_command 真进程，无假输出；
//      code === null = 非正常终了（超时强杀/信号），truncated 如实外显；
//   3. 桌面壳守卫 —— 非 Tauri 环境如实说明，不设假按钮；
//   4. 权限边界明示 —— 面板明示「以当前用户权限执行 / 不受审批链保护 /
//      非操作系统沙箱 / enable_shell 只关智能体工具面」（0 装：不许文案
//      暗示不存在的保护；口径见 docs/02-guides/permission-surfaces.md）；
//   5. mount 返回 cleanup（清理恰好一次）。

import type {AppContribution} from '../contribution-registry';

interface TerminalResult {
  code: number | null;
  stdout: string;
  stderr: string;
  duration_ms: number;
  truncated: boolean;
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

export function createTerminalApp(): AppContribution {
  return {
    id: 'starport-terminal',
    title: '舵室',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const note = el(doc, 'div', 'helm-note', '用户直操终端：工作目录钉死（仅钉当前目录）/ 30s 超时强杀直接子进程（树杀待接，孙进程可能遗留）/ 输出 256KB 截断（原生侧钉死）；智能体驱动走审批族（接线缝）。');
      const boundary = el(
        doc,
        'div',
        'helm-note',
        '权限边界：本终端以当前 Windows 用户权限执行命令，不受智能体审批链保护、不是操作系统沙箱；enable_shell=false（APEIRETH_ENABLE_SHELL=0，Shell 命令工具关）关闭的是智能体工具面，不影响此处。',
      );
      const wsInput = el(doc, 'input', 'helm-workspace');
      wsInput.placeholder = '工作区绝对路径';
      const cmdInput = el(doc, 'input', 'helm-command');
      cmdInput.placeholder = '命令（如：cargo --version）';
      const runBtn = el(doc, 'button', 'helm-run', '执行');
      const status = el(doc, 'div', 'helm-status');
      const output = el(doc, 'pre', 'helm-output');
      root.append(note, boundary, wsInput, cmdInput, runBtn, status, output);

      if (!isDesktop()) {
        status.textContent = '终端面需要桌面壳（当前为 Web 构建）——不设假按钮，执行禁用。';
        runBtn.disabled = true;
        return () => {
          root.textContent = '';
        };
      }

      void tauriInvoke<string | null>('get_workspace_dir')
        .then((dir) => {
          if (dir) wsInput.value = dir;
        })
        .catch(() => {
          status.textContent = '工作区目录读取失败（如实报）——请手动输入绝对路径。';
        });

      const run = async (): Promise<void> => {
        const command = cmdInput.value.trim();
        const workspaceDir = wsInput.value.trim();
        if (!command) {
          status.textContent = '命令为空。';
          return;
        }
        status.textContent = '执行中…';
        output.textContent = '';
        try {
          const result = await tauriInvoke<TerminalResult>('run_workspace_command', {
            command,
            workspaceDir,
          });
          output.textContent = `${result.stdout}${result.stderr ? `\n[stderr]\n${result.stderr}` : ''}`;
          const exitText = result.code === null ? '非正常终了（超时强杀/信号）' : `退出码 ${result.code}`;
          status.textContent = `${exitText} · ${result.duration_ms}ms${result.truncated ? ' · 输出已截断（256KB）' : ''}`;
        } catch (caught) {
          status.textContent = `执行失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      };
      runBtn.addEventListener('click', () => void run());

      return () => {
        root.textContent = '';
      };
    },
  };
}
