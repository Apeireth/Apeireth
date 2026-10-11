// 星港 · 内置件安装面（I 批 1）
//
// 契约：
//   1. hello 契约件是星港的「挂载契约证明」——证明注册表→壳→内置件→清理链
//      全链路真实可用（不是演示玩具，是活体契约件）；
//   2. 内置件 mount 只拿容器与上下文，DOM 经宿主容器内操作，不得越界；
//   3. 清理恰好一次：cleanup 清空容器内容，视图关闭链会调用它。
//
// 测试：tests/starport-registry.mjs / tests/starport-shell.mjs（真实模块直测）。

import type {AppContribution, AppMountContext, ContributionRegistry} from './contribution-registry';

export const STARPORT_HELLO_APP_ID = 'starport-hello';
export const STARPORT_HELLO_COMMAND_ID = 'starport-open-hello';

interface TextContainer {
  textContent?: string | null;
}

/**
 * hello 契约件：向容器写入挂载回执（内置件 id + 视图 id），
 * cleanup 清空容器。容器为宿主提供的真实元素（测试传桩对象）。
 */
export function createHelloApp(): AppContribution {
  return {
    id: STARPORT_HELLO_APP_ID,
    title: '星港回执',
    kind: 'internal',
    mount: (container: unknown, context: AppMountContext) => {
      const target = container as TextContainer;
      target.textContent = `星港在线：内置件 ${context.appId} 已挂载（视图 ${context.viewId}）。`;
      return () => {
        target.textContent = '';
      };
    },
  };
}

/**
 * 把星港批 1 内置件装进注册表。onOpenHello 由壳层注入（命令面板打开
 * hello 视图走 openOrFocusApp 同一链路）。
 */
export function installStarportBuiltins(
  registry: ContributionRegistry,
  options: {onOpenHello?: () => void} = {},
): void {
  registry.registerApp(createHelloApp());
  registry.registerCommand({
    id: STARPORT_HELLO_COMMAND_ID,
    title: '打开星港回执',
    handler: () => options.onOpenHello?.(),
  });
}
