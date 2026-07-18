use std::net::IpAddr;

use ipnet::IpNet;

/// ACL이 통제하는 동작. DB의 `acl_action` 열거와 짝이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclAction {
    Read,
    Edit,
    Move,
    Delete,
    CreateThread,
    WriteThreadComment,
    EditRequest,
    Acl,
}

impl AclAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Move => "move",
            Self::Delete => "delete",
            Self::CreateThread => "create_thread",
            Self::WriteThreadComment => "write_thread_comment",
            Self::EditRequest => "edit_request",
            Self::Acl => "acl",
        }
    }
}

/// 규칙이 무엇에 걸려 있는가. 문서 규칙이 이름공간 규칙보다 먼저 평가된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleScope {
    Document,
    Namespace,
}

/// 규칙의 조건. M2에서는 계정에 기대지 않는 것만 판정하고, 계정 조건(`perm:member`
/// 등)은 M3에서 채운다 — 그때까지는 매치되지 않아 다음 규칙으로 넘어간다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AclCondition {
    /// 누구나.
    Any,
    /// 비로그인 사용자.
    Ip,
    /// 로그인 사용자.
    Member,
    /// CIDR 범위.
    IpRange(String),
    /// 차단·경고 그룹 소속.
    AclGroup(String),
    /// 아직 판정하지 않는 조건(`user:`·`geoip:`·계정 기반 perm). 매치되지 않으므로
    /// 다음 규칙으로 넘어간다 — 모르는 조건을 허용으로 읽어 권한이 새는 것을 막는다.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclRule {
    pub scope: RuleScope,
    pub action: AclAction,
    pub evaluation_order: i64,
    pub condition: AclCondition,
    pub allowed: bool,
}

/// 판정 대상. 요청자가 누구이고 어떤 그룹에 속했는지의 스냅샷이다.
#[derive(Debug, Clone, Default)]
pub struct RequestSubject {
    pub ip_address: Option<String>,
    pub is_member: bool,
    pub acl_groups: Vec<String>,
}

impl RequestSubject {
    pub fn anonymous(ip_address: impl Into<String>) -> Self {
        Self {
            ip_address: Some(ip_address.into()),
            is_member: false,
            acl_groups: Vec::new(),
        }
    }

    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.acl_groups = groups;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclDecision {
    Allowed,
    Denied,
}

impl AclDecision {
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// the seed의 평가 순서를 그대로 따른다: 문서 규칙을 순서대로 보고 첫 매치의
/// 허용/거부로 즉시 결정, 매치가 없으면 이름공간 규칙, 그것도 없으면 거부.
///
/// 규칙이 하나도 없는 위키가 아무것도 못 하는 상태가 되지 않도록, 호출자는 기본
/// 규칙을 이름공간에 심어 둔다(마이그레이션 시드).
pub fn evaluate(rules: &[AclRule], action: AclAction, subject: &RequestSubject) -> AclDecision {
    let mut applicable: Vec<&AclRule> = rules.iter().filter(|rule| rule.action == action).collect();
    applicable.sort_by_key(|rule| (rule.scope, rule.evaluation_order));

    for rule in applicable {
        if matches(&rule.condition, subject) {
            return if rule.allowed {
                AclDecision::Allowed
            } else {
                AclDecision::Denied
            };
        }
    }

    AclDecision::Denied
}

fn matches(condition: &AclCondition, subject: &RequestSubject) -> bool {
    match condition {
        AclCondition::Any => true,
        AclCondition::Ip => !subject.is_member,
        AclCondition::Member => subject.is_member,
        AclCondition::IpRange(range) => subject
            .ip_address
            .as_deref()
            .zip(range.parse::<IpNet>().ok())
            .and_then(|(address, network)| {
                address
                    .parse::<IpAddr>()
                    .ok()
                    .map(|parsed| network.contains(&parsed))
            })
            .unwrap_or(false),
        AclCondition::AclGroup(name) => subject.acl_groups.iter().any(|group| group == name),
        AclCondition::Unsupported => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(scope: RuleScope, order: i64, condition: AclCondition, allowed: bool) -> AclRule {
        AclRule {
            scope,
            action: AclAction::Edit,
            evaluation_order: order,
            condition,
            allowed,
        }
    }

    #[test]
    fn 규칙이_없으면_거부한다() {
        let decision = evaluate(&[], AclAction::Edit, &RequestSubject::anonymous("1.2.3.4"));
        assert_eq!(decision, AclDecision::Denied);
    }

    #[test]
    fn 첫_매치에서_즉시_결정한다() {
        // 앞선 거부가 뒤따르는 허용을 이긴다.
        let rules = vec![
            rule(RuleScope::Document, 1, AclCondition::Ip, false),
            rule(RuleScope::Document, 2, AclCondition::Any, true),
        ];
        let decision = evaluate(
            &rules,
            AclAction::Edit,
            &RequestSubject::anonymous("1.2.3.4"),
        );
        assert_eq!(decision, AclDecision::Denied);
    }

    #[test]
    fn 문서_규칙이_이름공간_규칙보다_먼저다() {
        let rules = vec![
            rule(RuleScope::Namespace, 1, AclCondition::Any, false),
            rule(RuleScope::Document, 1, AclCondition::Any, true),
        ];
        let decision = evaluate(
            &rules,
            AclAction::Edit,
            &RequestSubject::anonymous("1.2.3.4"),
        );
        assert_eq!(decision, AclDecision::Allowed);
    }

    #[test]
    fn 문서_규칙이_매치되지_않으면_이름공간으로_넘어간다() {
        let rules = vec![
            rule(RuleScope::Document, 1, AclCondition::Member, true),
            rule(RuleScope::Namespace, 1, AclCondition::Any, false),
        ];
        // 비로그인이라 문서 규칙(member)은 매치되지 않는다.
        let decision = evaluate(
            &rules,
            AclAction::Edit,
            &RequestSubject::anonymous("1.2.3.4"),
        );
        assert_eq!(decision, AclDecision::Denied);
    }

    #[test]
    fn 다른_동작의_규칙은_보지_않는다() {
        let mut read_only = rule(RuleScope::Document, 1, AclCondition::Any, true);
        read_only.action = AclAction::Read;
        let decision = evaluate(
            &[read_only],
            AclAction::Edit,
            &RequestSubject::anonymous("1.2.3.4"),
        );
        assert_eq!(decision, AclDecision::Denied);
    }

    #[test]
    fn 대역_차단은_그_범위의_주소에만_걸린다() {
        let rules = vec![
            rule(
                RuleScope::Namespace,
                1,
                AclCondition::IpRange("192.168.0.0/16".to_owned()),
                false,
            ),
            rule(RuleScope::Namespace, 2, AclCondition::Any, true),
        ];

        assert_eq!(
            evaluate(
                &rules,
                AclAction::Edit,
                &RequestSubject::anonymous("192.168.1.5")
            ),
            AclDecision::Denied
        );
        assert_eq!(
            evaluate(
                &rules,
                AclAction::Edit,
                &RequestSubject::anonymous("10.0.0.1")
            ),
            AclDecision::Allowed
        );
    }

    #[test]
    fn 차단_그룹_소속이면_거부한다() {
        let rules = vec![
            rule(
                RuleScope::Namespace,
                1,
                AclCondition::AclGroup("차단된 사용자".to_owned()),
                false,
            ),
            rule(RuleScope::Namespace, 2, AclCondition::Any, true),
        ];

        let blocked =
            RequestSubject::anonymous("1.2.3.4").with_groups(vec!["차단된 사용자".to_owned()]);
        assert_eq!(
            evaluate(&rules, AclAction::Edit, &blocked),
            AclDecision::Denied
        );
        assert_eq!(
            evaluate(
                &rules,
                AclAction::Edit,
                &RequestSubject::anonymous("1.2.3.4")
            ),
            AclDecision::Allowed
        );
    }
}
