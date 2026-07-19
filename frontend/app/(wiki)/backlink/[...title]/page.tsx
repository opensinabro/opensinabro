import { Link } from "@/components/layout/link";
import { DocumentFrame } from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { linkStyle } from "@/components/ui/link";
import { Row, Rows } from "@/components/ui/list";
import { fetchBacklinks } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "역링크") };
}

export default async function BacklinkPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);

  // 역링크는 탭에 없는 도구 화면이라 어느 탭도 현재가 아니다 — 탭 줄은 그대로 걸어
  // 돌아가는 길이 다른 문서 화면과 같은 자리에 있게 한다.
  return (
    <DocumentFrame
      title={title}
      note="이 문서를 링크하거나 포함하는 문서"
      result={await fetchBacklinks(title)}
      denied="이 문서의 역링크를 볼 권한이 없습니다."
    >
      {({ entries }) =>
        entries.length === 0 ? (
          <Notice>이 문서를 가리키는 문서가 아직 없습니다.</Notice>
        ) : (
          <Rows>
            {entries.map((entry) => (
              <Row key={`${entry.kind}:${entry.title}`} shape="compact">
                <Link href={wikiPath.read(entry.title)} className={linkStyle()}>
                  {entry.title}
                </Link>
                <span className="text-faint">{entry.kindLabel}</span>
              </Row>
            ))}
          </Rows>
        )
      }
    </DocumentFrame>
  );
}
