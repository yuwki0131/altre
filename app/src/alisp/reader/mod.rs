use crate::alisp::ast::Expr;
use crate::alisp::error::{ReaderError, ReaderErrorKind, SourceLocation, SourceSpan};
use crate::alisp::symbol::SymbolInterner;

#[derive(Debug)]
pub struct Reader {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
}

impl Reader {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn parse(mut self, interner: &mut SymbolInterner) -> Result<Vec<Expr>, ReaderError> {
        let mut forms = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.is_eof() {
                break;
            }
            forms.push(self.read_form(interner)?);
        }
        Ok(forms)
    }

    fn read_form(&mut self, interner: &mut SymbolInterner) -> Result<Expr, ReaderError> {
        self.skip_whitespace_and_comments();
        let ch = self.peek_char().ok_or_else(|| self.error_here(ReaderErrorKind::UnexpectedEof, "入力が途中で終了しました"))?;
        if ch == '(' {
            self.consume_char();
            self.read_list(interner)
        } else if ch == '"' {
            self.read_string()
        } else if ch == '#' {
            self.read_boolean()
        } else if ch.is_ascii_digit() || (ch == '-' && self.peek_next().map_or(false, |c| c.is_ascii_digit())) {
            self.read_number()
        } else {
            self.read_symbol(interner)
        }
    }

    fn read_list(&mut self, interner: &mut SymbolInterner) -> Result<Expr, ReaderError> {
        let mut elements = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            match self.peek_char() {
                Some(')') => {
                    self.consume_char();
                    break;
                }
                Some(_) => {
                    let form = self.read_form(interner)?;
                    elements.push(form);
                }
                None => {
                    return Err(self.error_here(ReaderErrorKind::UnexpectedEof, "')' が不足しています"));
                }
            }
        }
        Ok(Expr::List(elements))
    }

    fn read_string(&mut self) -> Result<Expr, ReaderError> {
        let start = self.current_location();
        self.consume_char(); // opening quote
        let mut buf = String::new();
        while let Some(ch) = self.consume_char() {
            match ch {
                '"' => {
                    return Ok(Expr::String(buf));
                }
                '\\' => {
                    if let Some(escaped) = self.consume_char() {
                        let translated = match escaped {
                            'n' => '\n',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            other => {
                                return Err(self.error_at(start, ReaderErrorKind::UnexpectedChar(other), format!("未対応のエスケープ: \\{}", other)));
                            }
                        };
                        buf.push(translated);
                    } else {
                        return Err(self.error_at(start, ReaderErrorKind::UnterminatedString, "文字列リテラルが閉じられていません"));
                    }
                }
                other => buf.push(other),
            }
        }
        Err(self.error_at(start, ReaderErrorKind::UnterminatedString, "文字列リテラルが閉じられていません"))
    }

    fn read_boolean(&mut self) -> Result<Expr, ReaderError> {
        let start = self.current_location();
        self.consume_char(); // '#'
        let next = self.consume_char().ok_or_else(|| self.error_at(start, ReaderErrorKind::UnexpectedEof, "# の後に値がありません"))?;
        match next {
            't' | 'T' => Ok(Expr::Boolean(true)),
            'f' | 'F' => Ok(Expr::Boolean(false)),
            other => Err(self.error_at(start, ReaderErrorKind::UnexpectedChar(other), "真偽値リテラルは #t / #f のみ対応しています")),
        }
    }

    fn read_number(&mut self) -> Result<Expr, ReaderError> {
        let start = self.current_location();
        let mut buf = String::new();
        if self.peek_char() == Some('-') {
            buf.push('-');
            self.consume_char();
        }
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                buf.push(ch);
                self.consume_char();
            } else {
                break;
            }
        }
        let mut is_float = false;
        if self.peek_char() == Some('.') {
            if self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
                is_float = true;
                buf.push('.');
                self.consume_char();
                while let Some(ch) = self.peek_char() {
                    if ch.is_ascii_digit() {
                        buf.push(ch);
                        self.consume_char();
                    } else {
                        break;
                    }
                }
            }
        }
        if let Some(ch) = self.peek_char() {
            if matches!(ch, 'e' | 'E') {
                is_float = true;
                buf.push(ch);
                self.consume_char();
                if let Some(sign) = self.peek_char() {
                    if sign == '+' || sign == '-' {
                        buf.push(sign);
                        self.consume_char();
                    }
                }
                let mut has_digit = false;
                while let Some(d) = self.peek_char() {
                    if d.is_ascii_digit() {
                        buf.push(d);
                        self.consume_char();
                        has_digit = true;
                    } else {
                        break;
                    }
                }
                if !has_digit {
                    return Err(self.error_at(start, ReaderErrorKind::InvalidNumber(buf.clone()), "指数部に数字がありません"));
                }
            }
        }
        if is_float {
            buf.parse::<f64>()
                .map(Expr::Float)
                .map_err(|_| self.error_at(start, ReaderErrorKind::InvalidNumber(buf.clone()), "浮動小数の解析に失敗しました"))
        } else {
            buf.parse::<i64>()
                .map(Expr::Integer)
                .map_err(|_| self.error_at(start, ReaderErrorKind::InvalidNumber(buf.clone()), "整数の解析に失敗しました"))
        }
    }

    fn read_symbol(&mut self, interner: &mut SymbolInterner) -> Result<Expr, ReaderError> {
        let start = self.current_location();
        let mut buf = String::new();
        while let Some(ch) = self.peek_char() {
            if is_symbol_char(ch) {
                buf.push(ch);
                self.consume_char();
            } else {
                break;
            }
        }
        if buf.is_empty() {
            return Err(self.error_at(start, ReaderErrorKind::UnexpectedToken(String::new()), "シンボルを解析できませんでした"));
        }
        if buf == "nil" {
            return Err(self.error_at(start, ReaderErrorKind::UnexpectedToken(buf), "nil は v0 では利用できません"));
        }
        let id = interner.intern(&buf);
        Ok(Expr::Symbol(id))
    }

    fn skip_whitespace_and_comments(&mut self) -> bool {
        let mut progressed = false;
        loop {
            while let Some(ch) = self.peek_char() {
                if ch.is_whitespace() {
                    progressed = true;
                    self.consume_char();
                } else {
                    break;
                }
            }
            if self.peek_char() == Some(';') {
                progressed = true;
                self.consume_char();
                while let Some(ch) = self.peek_char() {
                    self.consume_char();
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
        progressed
    }

    fn consume_char(&mut self) -> Option<char> {
        let ch = self.chars.get(self.index).copied();
        if let Some(c) = ch {
            self.index += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        ch
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn is_eof(&self) -> bool {
        self.index >= self.chars.len()
    }

    fn current_location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column)
    }

    fn error_here(&self, kind: ReaderErrorKind, message: impl Into<String>) -> ReaderError {
        let loc = self.current_location();
        ReaderError::new(kind, SourceSpan::single_point(loc.line, loc.column), message)
    }

    fn error_at(&self, start: SourceLocation, kind: ReaderErrorKind, message: impl Into<String>) -> ReaderError {
        let end = self.current_location();
        ReaderError::new(kind, SourceSpan { start, end }, message)
    }
}

fn is_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '+' | '*' | '/' | '?' | '!')
}

pub fn parse(source: &str, interner: &mut SymbolInterner) -> Result<Vec<Expr>, ReaderError> {
    Reader::new(source).parse(interner)
}
