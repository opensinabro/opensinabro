//! `{{{#!if 조건식}}}`의 조건식 언어.
//!
//! 나무마크가 아니라 틀(include) 인자를 다루는 별도의 작은 표현식 언어다.
//! JavaScript를 닮았지만 훨씬 좁고, 나무위키 틀이 실제로 쓰는 만큼만 다룬다.
//!
//! ```text
//! {{{#!if top = 문서명1 != null ? 문서명1 : calleeTitle
//! 상위 문서: [[@top@]]}}}
//! ```
//!
//! 조건식은 값을 내는 동시에 **변수를 만든다**. 위에서 만든 `top`을 내용의 `@top@`이
//! 참조한다. 그래서 평가 결과로 참/거짓과 변수 바인딩을 함께 돌려준다.
//!
//! 문법(나무위키 틀에서 관찰된 범위):
//!
//! ```text
//! 시퀀스   a, b            (마지막 값이 결과)
//! 대입     이름 = 식
//! 삼항     조건 ? 참 : 거짓
//! 논리     a || b   a & b
//! 비교     a == b   a != b
//! 덧셈     a + b           (문자열 연결)
//! 후위     .length   .startsWith(x)   .substr(a[, b])   .lastIndexOf(x)   ?.
//! 기본     null   '문자열'   "문자열"   숫자   이름   ( 식 )
//! ```

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Text(String),
    Number(i64),
    Boolean(bool),
}

impl Value {
    /// JavaScript의 truthy 규칙을 따른다. `null`·빈 문자열·0·false가 거짓이다.
    fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Text(text) => !text.is_empty(),
            Value::Number(number) => *number != 0,
            Value::Boolean(boolean) => *boolean,
        }
    }

    fn to_text(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Text(text) => text.clone(),
            Value::Number(number) => number.to_string(),
            Value::Boolean(boolean) => boolean.to_string(),
        }
    }
}

/// 조건식을 평가한다. 조건이 참이면 `Some(바인딩)`, 거짓이면 `None`이다.
/// 식이 잘못됐으면 거짓으로 본다(틀이 깨져도 렌더는 계속되어야 한다).
pub(crate) fn evaluate(
    expression: &str,
    variables: &HashMap<String, String>,
) -> Option<HashMap<String, String>> {
    let mut evaluator = Evaluator {
        tokens: tokenize(expression)?,
        position: 0,
        scope: variables
            .iter()
            .map(|(name, value)| (name.clone(), Value::Text(value.clone())))
            .collect(),
    };
    let value = evaluator.sequence()?;
    if evaluator.position != evaluator.tokens.len() || !value.is_truthy() {
        return None;
    }
    Some(
        evaluator
            .scope
            .into_iter()
            .filter(|(_, value)| !matches!(value, Value::Null))
            .map(|(name, value)| (name, value.to_text()))
            .collect(),
    )
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Name(String),
    Text(String),
    Number(i64),
    Symbol(&'static str),
}

const SYMBOLS: [&str; 13] = [
    "?.", "==", "!=", "||", "&&", "&", "?", ":", ",", "=", "+", "(", ")",
];

fn tokenize(source: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let characters: Vec<char> = source.chars().collect();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character.is_whitespace() {
            index += 1;
            continue;
        }
        if character == '\'' || character == '"' {
            let mut text = String::new();
            index += 1;
            while index < characters.len() && characters[index] != character {
                text.push(characters[index]);
                index += 1;
            }
            if index >= characters.len() {
                return None; // 닫히지 않은 문자열
            }
            index += 1;
            tokens.push(Token::Text(text));
            continue;
        }
        if character.is_ascii_digit()
            || (character == '-' && characters.get(index + 1).is_some_and(char::is_ascii_digit))
        {
            let start = index;
            index += 1;
            while index < characters.len() && characters[index].is_ascii_digit() {
                index += 1;
            }
            let number: String = characters[start..index].iter().collect();
            tokens.push(Token::Number(number.parse().ok()?));
            continue;
        }
        if character == '.' {
            tokens.push(Token::Symbol("."));
            index += 1;
            continue;
        }
        if is_name_character(character) {
            let start = index;
            while index < characters.len() && is_name_character(characters[index]) {
                index += 1;
            }
            tokens.push(Token::Name(characters[start..index].iter().collect()));
            continue;
        }
        let rest: String = characters[index..].iter().collect();
        let symbol = SYMBOLS.iter().find(|symbol| rest.starts_with(**symbol))?;
        tokens.push(Token::Symbol(symbol));
        index += symbol.chars().count();
    }
    Some(tokens)
}

/// 틀 인자 이름에는 한글도 쓰인다(`문서명1`).
fn is_name_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

struct Evaluator {
    tokens: Vec<Token>,
    position: usize,
    scope: HashMap<String, Value>,
}

impl Evaluator {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn eat(&mut self, symbol: &str) -> bool {
        if matches!(self.peek(), Some(Token::Symbol(found)) if *found == symbol) {
            self.position += 1;
            return true;
        }
        false
    }

    fn sequence(&mut self) -> Option<Value> {
        let mut value = self.assignment()?;
        while self.eat(",") {
            value = self.assignment()?;
        }
        Some(value)
    }

    fn assignment(&mut self) -> Option<Value> {
        // `이름 = 식`인지 미리 살펴본다.
        if let Some(Token::Name(name)) = self.peek().cloned()
            && self.tokens.get(self.position + 1) == Some(&Token::Symbol("="))
        {
            self.position += 2;
            let value = self.assignment()?;
            self.scope.insert(name, value.clone());
            return Some(value);
        }
        self.ternary()
    }

    fn ternary(&mut self) -> Option<Value> {
        let condition = self.logical_or()?;
        if !self.eat("?") {
            return Some(condition);
        }
        let when_true = self.assignment()?;
        if !self.eat(":") {
            return None;
        }
        let when_false = self.assignment()?;
        Some(if condition.is_truthy() {
            when_true
        } else {
            when_false
        })
    }

    fn logical_or(&mut self) -> Option<Value> {
        let mut left = self.logical_and()?;
        while self.eat("||") {
            let right = self.logical_and()?;
            if !left.is_truthy() {
                left = right;
            }
        }
        Some(left)
    }

    /// 나무위키 틀은 논리 AND에 `&`를 쓴다(`문단 != null & 앵커 == null`). `&&`도 받는다.
    fn logical_and(&mut self) -> Option<Value> {
        let mut left = self.equality()?;
        loop {
            if !(self.eat("&&") || self.eat("&")) {
                return Some(left);
            }
            let right = self.equality()?;
            if left.is_truthy() {
                left = right;
            }
        }
    }

    fn equality(&mut self) -> Option<Value> {
        let mut left = self.addition()?;
        loop {
            let equal = if self.eat("==") {
                true
            } else if self.eat("!=") {
                false
            } else {
                return Some(left);
            };
            let right = self.addition()?;
            left = Value::Boolean(equals(&left, &right) == equal);
        }
    }

    fn addition(&mut self) -> Option<Value> {
        let mut left = self.postfix()?;
        while self.eat("+") {
            let right = self.postfix()?;
            left = match (&left, &right) {
                (Value::Number(left), Value::Number(right)) => Value::Number(left + right),
                _ => Value::Text(format!("{}{}", left.to_text(), right.to_text())),
            };
        }
        Some(left)
    }

    fn postfix(&mut self) -> Option<Value> {
        let mut value = self.primary()?;
        loop {
            // `?.`는 대상이 null이면 통째로 null이 된다.
            let optional = if self.eat("?.") {
                true
            } else if self.eat(".") {
                false
            } else {
                return Some(value);
            };
            let Some(Token::Name(member)) = self.peek().cloned() else {
                return None;
            };
            self.position += 1;
            if optional && value == Value::Null {
                // 인자 목록이 있으면 건너뛴다.
                if self.eat("(") {
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek()? {
                            Token::Symbol("(") => depth += 1,
                            Token::Symbol(")") => depth -= 1,
                            _ => {}
                        }
                        self.position += 1;
                    }
                }
                value = Value::Null;
                continue;
            }
            value = self.member(value, &member)?;
        }
    }

    fn member(&mut self, target: Value, member: &str) -> Option<Value> {
        if member == "length" {
            return Some(Value::Number(target.to_text().chars().count() as i64));
        }
        if !self.eat("(") {
            return None;
        }
        let mut arguments = Vec::new();
        if !self.eat(")") {
            loop {
                arguments.push(self.assignment()?);
                if self.eat(")") {
                    break;
                }
                if !self.eat(",") {
                    return None;
                }
            }
        }
        let text: Vec<char> = target.to_text().chars().collect();
        Some(match (member, arguments.as_slice()) {
            ("startsWith", [prefix]) => {
                Value::Boolean(target.to_text().starts_with(&prefix.to_text()))
            }
            ("endsWith", [suffix]) => Value::Boolean(target.to_text().ends_with(&suffix.to_text())),
            ("lastIndexOf", [needle]) => {
                Value::Number(char_index_of(&text, &needle.to_text(), true))
            }
            ("indexOf", [needle]) => Value::Number(char_index_of(&text, &needle.to_text(), false)),
            ("substr", [Value::Number(start)]) => {
                Value::Text(text.iter().skip(*start as usize).collect())
            }
            ("substr", [Value::Number(start), Value::Number(length)]) => Value::Text(
                text.iter()
                    .skip(*start as usize)
                    .take(*length as usize)
                    .collect(),
            ),
            _ => return None,
        })
    }

    fn primary(&mut self) -> Option<Value> {
        if self.eat("(") {
            let value = self.sequence()?;
            return self.eat(")").then_some(value);
        }
        let token = self.peek().cloned()?;
        self.position += 1;
        Some(match token {
            Token::Text(text) => Value::Text(text),
            Token::Number(number) => Value::Number(number),
            Token::Name(name) => match name.as_str() {
                "null" | "undefined" => Value::Null,
                "true" => Value::Boolean(true),
                "false" => Value::Boolean(false),
                _ => self.scope.get(&name).cloned().unwrap_or(Value::Null),
            },
            Token::Symbol(_) => return None,
        })
    }
}

/// 문자 단위 인덱스. JavaScript와 달리 바이트가 아니라 문자로 센다
/// (문서명에 한글이 흔하고, `substr`도 같은 기준으로 자른다).
fn char_index_of(text: &[char], needle: &str, from_end: bool) -> i64 {
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() || needle.len() > text.len() {
        return -1;
    }
    let candidates = 0..=(text.len() - needle.len());
    let found = if from_end {
        candidates
            .rev()
            .find(|start| text[*start..].starts_with(&needle))
    } else {
        candidates
            .into_iter()
            .find(|start| text[*start..].starts_with(&needle))
    };
    found.map(|index| index as i64).unwrap_or(-1)
}

fn equals(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Null, _) | (_, Value::Null) => false,
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::Boolean(left), Value::Boolean(right)) => left == right,
        _ => left.to_text() == right.to_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn simple_null_check() {
        assert!(evaluate("top2 != null", &scope(&[("top2", "문서")])).is_some());
        assert!(evaluate("top2 != null", &scope(&[])).is_none());
    }

    #[test]
    fn bare_name_is_truthy_check() {
        assert!(evaluate("n2", &scope(&[("n2", "값")])).is_some());
        assert!(evaluate("n2", &scope(&[("n2", "")])).is_none());
        assert!(evaluate("n2", &scope(&[])).is_none());
    }

    // 틀:상위 문서의 실제 조건식.
    #[test]
    fn parent_document_template() {
        let expression = "top = 문서명1 != null ? 문서명1 : calleeTitle != null ? (i = calleeTitle.lastIndexOf(\"/\")) != -1 ? calleeTitle.substr(0, i) : '상위 문서' : '상위 문서'";

        let bindings =
            evaluate(expression, &scope(&[("문서명1", "알파위키:문법 도움말")])).unwrap();
        assert_eq!(bindings.get("top").unwrap(), "알파위키:문법 도움말");

        // 인자가 없으면 호출자 문서명에서 상위 경로를 잘라 낸다.
        let bindings = evaluate(
            expression,
            &scope(&[("calleeTitle", "알파위키:문법 도움말/심화")]),
        )
        .unwrap();
        assert_eq!(bindings.get("top").unwrap(), "알파위키:문법 도움말");

        // `/`가 없으면 문자열 '상위 문서'가 된다.
        let bindings = evaluate(expression, &scope(&[("calleeTitle", "알파위키")])).unwrap();
        assert_eq!(bindings.get("top").unwrap(), "상위 문서");
    }

    // 틀:하위 문서 — 시퀀스로 변수를 여러 개 만들고 뒤에서 재사용한다.
    #[test]
    fn child_document_template() {
        let bindings = evaluate(
            "c = calleeTitle + \"/\", l = c.length, top1 = top1?.startsWith(c) ? top1.substr(l) : top1",
            &scope(&[("calleeTitle", "알파위키"), ("top1", "알파위키/마스코트")]),
        )
        .unwrap();
        assert_eq!(bindings.get("c").unwrap(), "알파위키/");
        assert_eq!(bindings.get("l").unwrap(), "5");
        assert_eq!(bindings.get("top1").unwrap(), "마스코트");
    }

    #[test]
    fn optional_chaining_on_null() {
        // top1이 없으면 `?.`가 통째로 null이 되어 조건이 거짓이다.
        assert!(
            evaluate(
                "top1?.startsWith(c) ? top1 : top1",
                &scope(&[("c", "알파위키/")])
            )
            .is_none()
        );
    }

    // 틀:상세 내용 — 괄호로 감싼 비교끼리 다시 비교한다.
    #[test]
    fn comparison_of_comparisons() {
        let expression = "(문단 == null) == (앵커 == null)";
        assert!(evaluate(expression, &scope(&[])).is_some());
        assert!(evaluate(expression, &scope(&[("문단", "3"), ("앵커", "개요")])).is_some());
        assert!(evaluate(expression, &scope(&[("문단", "3")])).is_none());
    }

    // 나무위키 틀은 논리 AND에 `&`를 쓴다.
    #[test]
    fn single_ampersand_is_logical_and() {
        assert!(evaluate("문단 != null & 앵커 == null", &scope(&[("문단", "3")])).is_some());
        assert!(
            evaluate(
                "문단 != null & 앵커 == null",
                &scope(&[("문단", "3"), ("앵커", "개요")])
            )
            .is_none()
        );
    }

    #[test]
    fn logical_or_falls_back() {
        let bindings = evaluate("n1 = n1 || '기본'", &scope(&[])).unwrap();
        assert_eq!(bindings.get("n1").unwrap(), "기본");
    }

    #[test]
    fn broken_expression_is_false() {
        assert!(evaluate("top = = ?", &scope(&[])).is_none());
        assert!(evaluate("'닫히지 않은", &scope(&[])).is_none());
    }
}
