-- 업로드할 때 고를 라이선스 목록. 운영자가 늘리고 줄이는 데이터다(docs/design/08).

INSERT INTO license (name, display_name, source_url) VALUES
    ('cc-by-nc-sa-2.0-kr', 'CC BY-NC-SA 2.0 KR',
     'https://creativecommons.org/licenses/by-nc-sa/2.0/kr/'),
    ('cc-by-sa-4.0', 'CC BY-SA 4.0',
     'https://creativecommons.org/licenses/by-sa/4.0/'),
    ('cc-by-4.0', 'CC BY 4.0', 'https://creativecommons.org/licenses/by/4.0/'),
    ('cc0', 'CC0 (퍼블릭 도메인 기증)', 'https://creativecommons.org/publicdomain/zero/1.0/'),
    ('public-domain', '퍼블릭 도메인', NULL),
    ('fair-use', '공정 이용', NULL);
