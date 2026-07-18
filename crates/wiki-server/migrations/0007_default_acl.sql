-- 평가는 "어느 규칙도 매치되지 않으면 거부"라(docs/design/08), 규칙이 하나도 없으면
-- 위키가 아무것도 못 하는 상태가 된다. 이름공간 기본값을 심어 열린 위키로 시작한다.
-- 운영자는 여기에 문서·이름공간 규칙을 더해 조이면 된다.

INSERT INTO acl_rule (namespace_id, action_id, evaluation_order, condition_kind_id, condition_value, allowed)
SELECT namespace.id, acl_action.id, 100, acl_condition_kind.id, 'any', true
FROM namespace
CROSS JOIN acl_action
JOIN acl_condition_kind ON acl_condition_kind.name = 'perm'
WHERE acl_action.name IN ('read', 'edit', 'create_thread', 'write_thread_comment', 'edit_request');

-- 이동·삭제·ACL 변경은 기본으로 열지 않는다 — 운영 권한이 붙는 M3에서 열린다.
INSERT INTO acl_rule (namespace_id, action_id, evaluation_order, condition_kind_id, condition_value, allowed)
SELECT namespace.id, acl_action.id, 100, acl_condition_kind.id, 'any', false
FROM namespace
CROSS JOIN acl_action
JOIN acl_condition_kind ON acl_condition_kind.name = 'perm'
WHERE acl_action.name IN ('move', 'delete', 'acl');

-- 차단 그룹은 어떤 기본 허용보다 먼저 평가되도록 순서를 앞에 둔다.
INSERT INTO acl_group (name, created_at) VALUES ('차단된 사용자', now());

INSERT INTO acl_rule (namespace_id, action_id, evaluation_order, condition_kind_id, condition_value, allowed)
SELECT namespace.id, acl_action.id, 10, acl_condition_kind.id, '차단된 사용자', false
FROM namespace
CROSS JOIN acl_action
JOIN acl_condition_kind ON acl_condition_kind.name = 'aclgroup'
WHERE acl_action.name IN ('edit', 'create_thread', 'write_thread_comment', 'edit_request');
