// 注册上面的加载钩子。`--import` 指向本文件即可生效：
//   node --import ./tests/ts-strip-register.mjs <suite>.mjs
//
// 见 `tests/ts-strip-hooks.mjs` 头部注释：**只在 Node 缺少内置 TS 支持时**才被用到。
import {register} from 'node:module';

register('./ts-strip-hooks.mjs', import.meta.url);
