//! 권한 판정을 소유한다.
//!
//! 평가는 저장소·HTTP와 분리된 순수 함수([`evaluate`])라, 규칙 조합의 의미를 단위
//! 테스트로 못박을 수 있다 (docs/design/07).

mod evaluate;
mod repository;

pub use evaluate::{
    AclAction, AclCondition, AclDecision, AclRule, RequestSubject, RuleScope, evaluate,
};
pub use repository::{
    AuthorizationError, Result, active_group_names, grant_permission, granted_permissions,
    load_rules, permission_log, revoke_permission,
};
