// SPDX-License-Identifier: GPL-3.0-or-later
//! Published numeric subset (§4.5), with binary64 accumulators.

pub(super) fn evaluate(text: &str) -> Result<f64, String> {
    let mut reader = Reader {
        input: text.as_bytes(),
        at: 0,
    };
    let value = reader.sum()?;
    reader.space();
    if reader.at != reader.input.len() {
        return Err("Bad real number".into());
    }
    Ok(value)
}

struct Reader<'a> {
    input: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn space(&mut self) {
        while self.input.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn take(&mut self, byte: u8) -> bool {
        self.space();
        if self.input.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn sum(&mut self) -> Result<f64, String> {
        let mut left = self.product()?;
        loop {
            if self.take(b'+') {
                left += self.product()?;
            } else if self.take(b'-') {
                left -= self.product()?;
            } else {
                return Ok(left);
            }
        }
    }

    fn product(&mut self) -> Result<f64, String> {
        let mut left = self.power()?;
        loop {
            if self.take(b'*') {
                left *= self.power()?;
            } else if self.take(b'/') {
                left /= self.power()?;
            } else if self.take(b'%') {
                let right = self.power()?;
                let quotient = left / right;
                left -= quotient.floor() * right;
            } else {
                return Ok(left);
            }
        }
    }

    fn power(&mut self) -> Result<f64, String> {
        let left = self.factor()?;
        if self.take(b'^') {
            Ok(left.powf(self.power()?))
        } else {
            Ok(left)
        }
    }

    fn factor(&mut self) -> Result<f64, String> {
        if self.take(b'+') {
            return self.factor();
        }
        if self.take(b'-') {
            return Ok(-self.factor()?);
        }
        if self.take(b'(') {
            let value = self.sum()?;
            if !self.take(b')') {
                return Err("Bad real number".into());
            }
            return Ok(value);
        }
        self.space();
        if self.input.get(self.at).is_some_and(u8::is_ascii_alphabetic) {
            // GR4: full primary/function/domain tables remain explicitly open.
            return Err("Unsupported GR4 numeric primary/function".into());
        }
        let start = self.at;
        if self.input.get(self.at..self.at + 2) == Some(b"0x") {
            self.at += 2;
            let hex = self.at;
            while self.input.get(self.at).is_some_and(u8::is_ascii_hexdigit) {
                self.at += 1;
            }
            let text = std::str::from_utf8(&self.input[hex..self.at]).map_err(|_| "Bad real number")?;
            return u32::from_str_radix(text, 16)
                .map(f64::from)
                .map_err(|_| "Unsupported GR4 hexadecimal conversion".into());
        }
        while self.input.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        if self.input.get(self.at) == Some(&b'.') {
            self.at += 1;
            while self.input.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        if matches!(self.input.get(self.at), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.input.get(self.at), Some(b'+' | b'-')) {
                self.at += 1;
            }
            while self.input.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        std::str::from_utf8(&self.input[start..self.at])
            .ok()
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| "Bad real number".into())
    }
}
