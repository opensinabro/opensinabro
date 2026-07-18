/// 두 리비전 사이의 줄 단위 변경 한 덩어리.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Inserted,
    Deleted,
}

/// 편집 충돌을 자동 병합한 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// 겹치지 않아 합쳐졌다.
    Merged(String),
    /// 같은 자리를 서로 다르게 고쳐 사람이 골라야 한다. 충돌 표시가 들어 있다.
    Conflicted(String),
}

/// 두 원문의 줄 단위 차이.
pub fn diff_lines(before: &str, after: &str) -> Vec<DiffLine> {
    diffy::create_patch(before, after)
        .hunks()
        .iter()
        .flat_map(|hunk| hunk.lines())
        .map(|line| {
            let (kind, text) = match line {
                diffy::Line::Context(text) => (DiffLineKind::Context, text),
                diffy::Line::Insert(text) => (DiffLineKind::Inserted, text),
                diffy::Line::Delete(text) => (DiffLineKind::Deleted, text),
            };
            DiffLine {
                kind,
                text: text.trim_end_matches('\n').to_owned(),
            }
        })
        .collect()
}

/// 편집 충돌 병합.
///
/// 편집자가 편집을 시작한 시점의 원문(`base`)을 기준으로, 그 사이 저장된 현재
/// 원문(`current`)과 편집자가 낸 원문(`proposed`)을 합친다. 서로 다른 자리를
/// 고쳤으면 충돌 없이 합쳐진다.
pub fn merge_edits(base: &str, current: &str, proposed: &str) -> MergeOutcome {
    match diffy::merge(base, current, proposed) {
        Ok(merged) => MergeOutcome::Merged(merged),
        Err(conflicted) => MergeOutcome::Conflicted(conflicted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 바뀐_줄만_차이로_잡는다() {
        let diff = diff_lines("첫 줄\n둘째 줄\n", "첫 줄\n바뀐 줄\n");

        assert!(diff.contains(&DiffLine {
            kind: DiffLineKind::Deleted,
            text: "둘째 줄".to_owned()
        }));
        assert!(diff.contains(&DiffLine {
            kind: DiffLineKind::Inserted,
            text: "바뀐 줄".to_owned()
        }));
    }

    #[test]
    fn 서로_다른_자리를_고치면_충돌_없이_합쳐진다() {
        let base = "머리말\n본문\n맺음말\n";
        let current = "바뀐 머리말\n본문\n맺음말\n";
        let proposed = "머리말\n본문\n바뀐 맺음말\n";

        let MergeOutcome::Merged(merged) = merge_edits(base, current, proposed) else {
            panic!("겹치지 않는 편집은 합쳐져야 한다");
        };
        assert!(merged.contains("바뀐 머리말"));
        assert!(merged.contains("바뀐 맺음말"));
    }

    #[test]
    fn 같은_자리를_다르게_고치면_충돌이다() {
        let base = "본문\n";
        let current = "이쪽으로 고침\n";
        let proposed = "저쪽으로 고침\n";

        assert!(matches!(
            merge_edits(base, current, proposed),
            MergeOutcome::Conflicted(_)
        ));
    }

    #[test]
    fn 한쪽만_고쳤으면_그대로_반영한다() {
        let base = "본문\n";
        let merged = merge_edits(base, base, "새 본문\n");
        assert_eq!(merged, MergeOutcome::Merged("새 본문\n".to_owned()));
    }
}
