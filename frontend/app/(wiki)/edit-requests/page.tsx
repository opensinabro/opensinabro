import { Link } from "@/components/layout/link";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchEditRequests } from "@/lib/api/discussion";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";
import { linkStyle } from "@/components/ui/link";
import { Row, Rows } from "@/components/ui/list";

export const metadata = { title: pageTitle("편집요청") };

export default async function EditRequestsPage() {
  const result = await fetchEditRequests();
  const requests = result.kind === "found" ? result.data.requests : [];

  return (
    <WikiPage
      header={<PageHeader title="편집요청" note="아직 처리되지 않은 변경안" />}
    >
      {requests.length === 0 ? (
        <Notice>열린 편집요청이 없습니다.</Notice>
      ) : (
        <Rows>
          {requests.map((request) => (
            <Row key={request.id}>
              <Link
                href={wikiPath.editRequest(request.id)}
                className={linkStyle()}
              >
                {request.title}
              </Link>
              <span className="text-muted">{request.author}</span>
              {request.comment && (
                <span className="text-body">{request.comment}</span>
              )}
            </Row>
          ))}
        </Rows>
      )}
    </WikiPage>
  );
}
