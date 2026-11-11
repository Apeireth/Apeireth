//! 浏览器实现（`Intl` 口径 ⇒ 与生产**逐字同源**）。

use apeireth_ui_kit::governance_tab::TimeTextFormatter;
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

/// 浏览器时刻文本（`Intl` 口径）。
pub struct BrowserTimeText;

impl BrowserTimeText {
    /// 建一个 `Date` 对象（`new Date(ts)`）。
    ///
    /// ⚠️ 用 `Reflect::construct`（而不是 `Function::new_with_arg_1`）：
    /// 后者在 `js-sys 0.3` **不存在**（`Function` 没有构造器 API）⇒
    /// 而 `Reflect::construct` 正是 `new X(args)` 的通用反射形式。
    fn new_date(ts_ms: i64) -> Option<Object> {
        let global = js_sys::global();
        let date_ctor = Reflect::get(&global, &JsValue::from_str("Date")).ok()?;
        // ⚠️ `Reflect::construct` 要的是 `&Function`（不是 `&JsValue`）。
        let ctor: Function = date_ctor.dyn_into().ok()?;
        let array = Array::new();
        array.push(&JsValue::from_f64(ts_ms as f64));
        let value = Reflect::construct(&ctor, &array).ok()?;
        value.dyn_into::<Object>().ok()
    }

    /// 调对象上的方法（`obj.method(args…)`）⇒ 返回**原始值**。
    ///
    /// ⚠️ 为什么要有「原始值」这一层：有些方法返回**数字**（`getTime()`）⇒
    /// 而 `as_string()` 对数字返回 `None` ⇒ ⇒ 若把 `None` 当「调用失败」
    /// ⇒ ⇒ 合法日期会被误判为无效 ⇒ ⇒ 输出 `—`（实测踩过）。
    fn call_raw(object: &Object, name: &str, args: &[JsValue]) -> Option<JsValue> {
        let method = Reflect::get(object, &JsValue::from_str(name)).ok()?;
        // ⚠️ `JsCast::dyn_into` 返回 `Result`（不是 `Option`）⇒ 要 `.ok()?`。
        let function: Function = method.dyn_into().ok()?;
        // ⚠️ `Function::apply` 要的是 `&Array`（不是 `&[JsValue]`）。
        let array = Array::new();
        for arg in args {
            array.push(arg);
        }
        function.apply(object.as_ref(), &array).ok()
    }

    /// 调对象上的方法 ⇒ 取**字符串**结果。
    fn call_method(object: &Object, name: &str, args: &[JsValue]) -> Option<String> {
        Self::call_raw(object, name, args)?.as_string()
    }

    /// `date.toLocaleTimeString('zh-CN', options)`。
    fn locale_time(date: &Object, second: bool) -> Option<String> {
        let options = Object::new();
        for key in ["hour", "minute"] {
            Reflect::set(
                &options,
                &JsValue::from_str(key),
                &JsValue::from_str("2-digit"),
            )
            .ok()?;
        }
        if second {
            Reflect::set(
                &options,
                &JsValue::from_str("second"),
                &JsValue::from_str("2-digit"),
            )
            .ok()?;
        }
        Self::call_method(
            date,
            "toLocaleTimeString",
            &[JsValue::from_str("zh-CN"), options.into()],
        )
    }
}

impl TimeTextFormatter for BrowserTimeText {
    fn clock(&self, ts_ms: i64) -> String {
        // ⚠️ **逐字照抄生产 `formatClockMs`**（`governance/format.ts`）：
        // `Number.isNaN(d.getTime())` ⇒ `'—'`；否则 `toLocaleTimeString('zh-CN', …)`。
        let Some(date) = Self::new_date(ts_ms) else {
            return String::from("—");
        };
        // ⚠️ **逐字照抄生产的 `Number.isNaN(d.getTime())`**：
        // `getTime()` 返回**数字** ⇒ 必须按原始值判 `NaN`
        // ⇒ （不能把「取字符串失败」当无效 ⇒ 那会把合法日期误判成 `—`）。
        let Some(value) = Self::call_raw(&date, "getTime", &[]) else {
            return String::from("—");
        };
        match value.as_f64() {
            Some(number) if !number.is_nan() => {}
            _ => return String::from("—"),
        }
        Self::locale_time(&date, true).unwrap_or_else(|| String::from("—"))
    }

    fn date_time(&self, ts_ms: i64, now_ms: i64) -> String {
        // ⚠️ **逐字照抄生产 `formatDateTimeMs`**：
        // `sameDay = d.toDateString() === today.toDateString()` ⇒ 走**本地时区**；
        // 同日 ⇒ 只给时刻；跨日 ⇒
        // `toLocaleDateString('zh-CN', {month:'2-digit', day:'2-digit'})` + 空格 + 时刻。
        let (Some(date), Some(today)) = (Self::new_date(ts_ms), Self::new_date(now_ms)) else {
            return String::from("—");
        };
        let Some(date_stamp) = Self::call_method(&date, "toDateString", &[]) else {
            return String::from("—");
        };
        let today_stamp = Self::call_method(&today, "toDateString", &[]).unwrap_or_default();
        let Some(hm) = Self::locale_time(&date, false) else {
            return String::from("—");
        };
        if date_stamp == today_stamp {
            return hm;
        }
        let options = Object::new();
        for key in ["month", "day"] {
            if Reflect::set(
                &options,
                &JsValue::from_str(key),
                &JsValue::from_str("2-digit"),
            )
            .is_err()
            {
                // ⚠️ 写不进选项 ⇒ 退回「只给时刻」（**不假装成功**）。
                return hm;
            }
        }
        let Some(date_text) = Self::call_method(
            &date,
            "toLocaleDateString",
            &[JsValue::from_str("zh-CN"), options.into()],
        ) else {
            return hm;
        };
        format!("{date_text} {hm}")
    }
}
