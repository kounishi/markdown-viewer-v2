//! 文字コード判別: BOM → 厳密 UTF-8 → Shift-JIS フォールバック (参考アプリ ReadAllTextSmart と同等)。

pub struct Decoded {
    pub text: String,
    pub encoding: &'static str,
}

pub fn decode(bytes: &[u8]) -> Decoded {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return Decoded {
            text: String::from_utf8_lossy(rest).into_owned(),
            encoding: "UTF-8 (BOM)",
        };
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let (text, _) = encoding_rs::UTF_16LE.decode_without_bom_handling(rest);
        return Decoded { text: text.into_owned(), encoding: "UTF-16 LE" };
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let (text, _) = encoding_rs::UTF_16BE.decode_without_bom_handling(rest);
        return Decoded { text: text.into_owned(), encoding: "UTF-16 BE" };
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Decoded { text: text.to_owned(), encoding: "UTF-8" },
        Err(_) => {
            let (text, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
            Decoded { text: text.into_owned(), encoding: "Shift_JIS" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn plain_utf8() {
        let d = decode("こんにちは".as_bytes());
        assert_eq!(d.text, "こんにちは");
        assert_eq!(d.encoding, "UTF-8");
    }

    #[test]
    fn utf8_with_bom_strips_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("abc".as_bytes());
        let d = decode(&bytes);
        assert_eq!(d.text, "abc");
        assert_eq!(d.encoding, "UTF-8 (BOM)");
    }

    #[test]
    fn utf16le_with_bom() {
        // "あ" = U+3042 → LE: 42 30
        let d = decode(&[0xFF, 0xFE, 0x42, 0x30]);
        assert_eq!(d.text, "あ");
        assert_eq!(d.encoding, "UTF-16 LE");
    }

    #[test]
    fn utf16be_with_bom() {
        let d = decode(&[0xFE, 0xFF, 0x30, 0x42]);
        assert_eq!(d.text, "あ");
        assert_eq!(d.encoding, "UTF-16 BE");
    }

    #[test]
    fn shift_jis_fallback() {
        // "日本語" in Shift_JIS
        let d = decode(&[0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA]);
        assert_eq!(d.text, "日本語");
        assert_eq!(d.encoding, "Shift_JIS");
    }
}
