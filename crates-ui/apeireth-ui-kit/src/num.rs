//! 脚本侧数值 / 文本语义（**零依赖**手写实现）。
//!
//! ## 为什么要自己实现
//!
//! 卡面上的时长 / 延迟文本由**脚本侧**的数值格式化产出（`toFixed(1)` /
//! `Math.round`），JSON 序列化也要与脚本侧的 `JSON.stringify` **逐字节相同**
//! ⇒ 这些语义必须逐条对齐 ⇒ 这里手写一份**可测**的实现（通用库给不了这些
//! 语义，且本仓 P0 纪律是 0 外部依赖）。
//!
//! ## 对齐了哪些语义
//!
//! | 函数 | 语义 | 与原生做法的差别 |
//! |---|---|---|
//! | [`number_to_string`] | 数值 ⇒ 文本 | 原生 `{}` **不用**指数记法（`1e21` 会写成 21 位整数） |
//! | [`to_fixed`] | 保留固定小数位 | 原生 `{:.1}` 是**半数取偶**（`1.25 → 1.2`），脚本侧是**半数进位**（`1.25 → 1.3`） |
//! | [`round`] | 取整 | 原生 `f64::round` 是**半数远离零**，脚本侧是**半数向 +∞** |
//! | [`string_length`] | 文本长度 | 按 **UTF-16 码元**计（不是字节、也不是字符数） |
//! | [`string_slice`] | 文本切片 | 按 **UTF-16 码元**切（与脚本侧 `slice` 同口径） |
//!
//! ## 边界（明确登记）
//!
//! - [`string_slice`] 在**代理对**（非 BMP 字符）被切开的边界上，丢弃整个代理对
//!   （脚本侧会留下半个代理对）。本仓固定数据**不含**非 BMP 字符 ⇒ 该边界
//!   在当前数据下**不可达**；一旦引入非 BMP 字符，须改用 `u16` 序列切片。
//! - [`to_fixed`] 在数值 ≥ 1e21 时按 [`number_to_string`] 的记法输出（与脚本侧一致）。
//! - 非有限数值（NaN / ±∞）不参与 JSON 序列化（按脚本侧语义写成 `null`）。

/// 数值 ⇒ 文本（十进制 / 指数记法，与脚本侧的数值转文本语义一致）。
///
/// ## 算法
///
/// 取**最短往返十进制**（`{:e}`）⇒ 拆成数字串与十进制指数 ⇒ 按脚本侧的
/// 分支规则排版：
///
/// | 条件 | 记法 |
/// |---|---|
/// | `k ≤ n ≤ 21` | 数字串 + `n − k` 个零 |
/// | `0 < n ≤ 21` | 数字串前 `n` 位 + `.` + 其余 |
/// | `−6 < n ≤ 0` | `0.` + `−n` 个零 + 数字串 |
/// | 其余 | 指数记法（`1e+21` / `1e-7`） |
///
/// （`k` = 数字位数，`n` = 十进制指数 + 1。）
pub fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    if value == 0.0 {
        // 含 -0 ⇒ 文本是 "0"。
        return String::from("0");
    }
    if value.is_infinite() {
        return if value < 0.0 {
            String::from("-Infinity")
        } else {
            String::from("Infinity")
        };
    }
    if value < 0.0 {
        return format!("-{}", number_to_string(-value));
    }
    // 最短往返十进制 ⇒ "d.ddde±E"（指数无前导零、无正号）。
    let repr = format!("{value:e}");
    let Some((mantissa, exponent)) = repr.split_once('e') else {
        return repr;
    };
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|ch| *ch != '.').collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let count = digits.len() as i32;
    let point = exponent + 1;
    if count <= point && point <= 21 {
        let mut out = String::from(digits);
        out.push_str(&"0".repeat((point - count) as usize));
        out
    } else if 0 < point && point <= 21 {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    } else if -6 < point && point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), digits)
    } else {
        let power = point - 1;
        let sign = if power >= 0 { "+" } else { "-" };
        if count == 1 {
            format!("{digits}e{sign}{}", power.abs())
        } else {
            format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], power.abs())
        }
    }
}

/// 十进制大整数（**最高位在前**的十进制数字数组；无前导零）。
///
/// 只用于「精确十进制展开」⇒ 需要的运算：乘小整数、取十进制数字串、按 10 的幂
/// 切分。
struct Decimal {
    /// 十进制数字（最高位在前）。
    digits: Vec<u8>,
}

impl Decimal {
    /// 由无符号整数建。
    fn from_u64(value: u64) -> Self {
        let text = value.to_string();
        let digits = text.bytes().map(|byte| byte - b'0').collect();
        Self { digits }
    }

    /// 乘一个小整数（就地）。
    fn mul_small(&mut self, factor: u32) {
        let mut carry = 0u32;
        for digit in self.digits.iter_mut().rev() {
            let product = u32::from(*digit) * factor + carry;
            *digit = (product % 10) as u8;
            carry = product / 10;
        }
        while carry > 0 {
            self.digits.insert(0, (carry % 10) as u8);
            carry /= 10;
        }
    }

    /// 十进制数字串（最高位在前）。
    fn to_text(&self) -> String {
        self.digits
            .iter()
            .map(|digit| char::from(b'0' + digit))
            .collect()
    }
}

/// 浮点数的**精确**十进制展开 ⇒ `(数字串, 小数位数)`（值 = 数字串 × 10^-小数位数）。
///
/// 浮点数是二进制有限小数 ⇒ 乘 5 的幂即可得到精确十进制展开（无误差、无近似）。
fn exact_decimal(value: f64) -> (Decimal, u32) {
    let bits = value.to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if exponent_bits == 0 {
        // 次正规数。
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), exponent_bits - 1075)
    };
    if exponent >= 0 {
        let mut digits = Decimal::from_u64(mantissa);
        for _ in 0..exponent {
            digits.mul_small(2);
        }
        (digits, 0)
    } else {
        let power = (-exponent) as u32;
        let mut digits = Decimal::from_u64(mantissa);
        for _ in 0..power {
            digits.mul_small(5);
        }
        (digits, power)
    }
}

/// 保留固定小数位（**精确值**四舍五入，半数进位）。
///
/// ## 与原生 `{:.N}` 的差别
///
/// 原生是**半数取偶**（`1.25 → "1.2"`），脚本侧是**半数进位**（`1.25 → "1.3"`）
/// ⇒ 这里按**精确十进制展开**做进位判定 ⇒ 不依赖最短往返近似。
pub fn to_fixed(value: f64, digits: usize) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    if value.is_infinite() {
        return number_to_string(value);
    }
    if value < 0.0 {
        return format!("-{}", to_fixed(-value, digits));
    }
    if value == 0.0 {
        return if digits == 0 {
            String::from("0")
        } else {
            format!("0.{}", "0".repeat(digits))
        };
    }
    let (exact, scale) = exact_decimal(value);
    let scale = scale as usize;
    if scale <= digits {
        // 精确展开的小数位不足 ⇒ 补零。
        let mut text = exact.to_text();
        text.push_str(&"0".repeat(digits - scale));
        return insert_point(&text, digits);
    }
    // 丢掉 `scale - digits` 位 ⇒ 四舍五入（半数进位）。
    let drop = scale - digits;
    let text = exact.to_text();
    let bytes = text.as_bytes();
    let keep = bytes.len() - drop;
    let round_up = keep < bytes.len() && bytes[keep] >= b'5';
    let mut head: Vec<u8> = text.as_bytes()[..keep].to_vec();
    if round_up {
        // 进位（可能一路进位，例如 "999" → "1000"）。
        let mut carry = 1u8;
        let mut index = head.len();
        while carry > 0 && index > 0 {
            index -= 1;
            let sum = head[index] - b'0' + carry;
            carry = sum / 10;
            head[index] = b'0' + (sum % 10);
        }
        if carry > 0 {
            head.insert(0, b'0' + carry);
        }
    }
    let head = String::from_utf8(head).unwrap_or_default();
    insert_point(&head, digits)
}

/// 在数字串的右侧数 `digits` 位处插小数点（不足则在前面补零）。
fn insert_point(text: &str, digits: usize) -> String {
    if digits == 0 {
        return text.to_owned();
    }
    if text.len() <= digits {
        // 整数部分为 0 ⇒ "0." + 补零 + 小数部分。
        let padding = "0".repeat(digits - text.len());
        return format!("0.{padding}{text}");
    }
    let split = text.len() - digits;
    format!("{}.{}", &text[..split], &text[split..])
}

/// 取整（**半数向 +∞**，与脚本侧的 `Math.round` 同口径）。
///
/// 原生 `f64::round` 是**半数远离零**（`-2.5 → -3`）⇒ 这里按地板 + 余数判定。
pub fn round(value: f64) -> f64 {
    if value.is_nan() || value.is_infinite() {
        return value;
    }
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// 文本长度（按 **UTF-16 码元**计，与脚本侧的 `String.length` 同口径）。
pub fn string_length(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// 文本切片（按 **UTF-16 码元**切，与脚本侧的 `slice` 同口径）。
///
/// ## 边界
///
/// 代理对（非 BMP 字符）被切开时**丢弃整个代理对**（脚本侧会留下半个代理对）。
/// 本仓固定数据不含非 BMP 字符 ⇒ 该边界在当前数据下不可达。
pub fn string_slice(text: &str, start: usize, end: usize) -> String {
    let mut out = String::new();
    let mut index = 0usize;
    for ch in text.chars() {
        let width = ch.len_utf16();
        if index >= end {
            break;
        }
        if index >= start && index + width <= end {
            out.push(ch);
        }
        index += width;
    }
    out
}

#[cfg(test)]
mod tests {
    //! 数值 / 文本语义的断言（**行为**，不是源码镜像）。

    use super::*;

    #[test]
    fn integers_are_written_without_a_fractional_part() {
        assert_eq!(number_to_string(0.0), "0");
        assert_eq!(number_to_string(1.0), "1");
        assert_eq!(number_to_string(42.0), "42");
        assert_eq!(number_to_string(-7.0), "-7");
        assert_eq!(number_to_string(1_500.0), "1500");
    }

    #[test]
    fn fractions_use_the_shortest_round_trip_form() {
        assert_eq!(number_to_string(1.5), "1.5");
        assert_eq!(number_to_string(0.1), "0.1");
        assert_eq!(number_to_string(1.25), "1.25");
        assert_eq!(number_to_string(0.042), "0.042");
        assert_eq!(number_to_string(-0.5), "-0.5");
    }

    #[test]
    fn exponential_notation_is_used_outside_the_decimal_window() {
        // 1e21 ⇒ 指数记法（脚本侧同）。
        assert_eq!(number_to_string(1e21), "1e+21");
        assert_eq!(number_to_string(1e-7), "1e-7");
        // 1e20 仍在十进制窗口内。
        assert_eq!(number_to_string(1e20), "100000000000000000000");
        assert_eq!(number_to_string(1e-6), "0.000001");
    }

    #[test]
    fn non_finite_values_use_their_own_text() {
        assert_eq!(number_to_string(f64::NAN), "NaN");
        assert_eq!(number_to_string(f64::INFINITY), "Infinity");
        assert_eq!(number_to_string(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(number_to_string(-0.0), "0");
    }

    #[test]
    fn fixed_digits_round_half_up_on_the_exact_value() {
        assert_eq!(to_fixed(1.5, 1), "1.5");
        assert_eq!(to_fixed(2.0, 1), "2.0");
        assert_eq!(to_fixed(0.042, 1), "0.0");
        assert_eq!(to_fixed(0.087, 1), "0.1");
        assert_eq!(to_fixed(1.25, 1), "1.3");
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(-1.25, 1), "-1.3");
    }

    #[test]
    fn fixed_digits_pad_and_truncate_like_the_script_side() {
        assert_eq!(to_fixed(0.0, 2), "0.00");
        assert_eq!(to_fixed(1.0, 0), "1");
        assert_eq!(to_fixed(1234.5678, 2), "1234.57");
        assert_eq!(to_fixed(0.5, 0), "1");
        assert_eq!(to_fixed(1.5, 0), "2");
    }

    #[test]
    fn rounding_goes_towards_positive_infinity_on_ties() {
        assert_eq!(round(0.5), 1.0);
        assert_eq!(round(-0.5), 0.0);
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(2.4), 2.0);
        assert_eq!(round(-2.6), -3.0);
    }

    #[test]
    fn string_length_counts_utf16_code_units() {
        assert_eq!(string_length("abc"), 3);
        // 一个汉字 = 一个 UTF-16 码元（三个字节）。
        assert_eq!(string_length("卷宗"), 2);
        // 字节数与码元数**不同**（汉字 = 1 码元 / 3 字节）。
        assert_eq!("卷宗".len(), 6);
    }

    #[test]
    fn string_slice_uses_utf16_offsets() {
        assert_eq!(string_slice("卷宗工作区", 0, 2), "卷宗");
        assert_eq!(string_slice("卷宗工作区", 2, 6), "工作区");
        assert_eq!(string_slice("abcdef", 1, 3), "bc");
        assert_eq!(string_slice("abcdef", 4, 2), "");
    }
}
