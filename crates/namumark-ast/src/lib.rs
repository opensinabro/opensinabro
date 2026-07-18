//! 나무마크 의미 모델.
//!
//! 무손실 구문 트리([`namumark_syntax`]) 위의 타입 뷰다 — 각 노드 타입은 `SyntaxNode`
//! 하나를 감싸고, 접근자가 문법이 끊어 놓은 토큰을 읽어 의미값을 계산한다. 원문을 다시
//! 쪼개지 않으므로 파싱은 구문 계층에서 한 번만 일어난다. 원문 위치·토큰이 필요하면
//! [`AstNode::syntax`]로 구문 노드를 그대로 쓴다.

mod node;
mod value;

pub use node::{
    AstNode, Block, Bold, Category, CodeBlock, ColoredText, Comment, Conditional, Document, Folding,
    Footnote, Heading, Image, Indent, Inline, Italic, Link, List, ListItem, Macro, Paragraph,
    Quote, Redirect, SizedText, Strikethrough, Subscript, Superscript, Table, TableCell, TableRow,
    Underline, WikiStyle, parse, template_of,
};
pub use value::{
    Fragment, HorizontalAlignment, ImageOption, ListKind, TableAttribute, TableAttributeScope,
    Template, Variable, VerticalAlignment,
};
