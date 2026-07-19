import { Link } from "@/components/layout/link";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { MarkAllReadButton } from "@/components/layout/mark-all-read-button";
import { fetchNotifications } from "@/lib/api/special";
import { formatMoment } from "@/lib/format";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";
import { linkStyle } from "@/components/ui/link";
import { Row, Rows } from "@/components/ui/list";

export const metadata = { title: pageTitle("알림") };

export default async function NotificationsPage() {
  const result = await fetchNotifications();

  if (result.kind === "unauthorized") {
    return (
      <WikiPage header={<PageHeader title="알림" />}>
        <Notice>
          로그인해야 볼 수 있는 화면입니다.{" "}
          <Link href="/login" className={linkStyle()}>
            로그인하기
          </Link>
        </Notice>
      </WikiPage>
    );
  }

  if (result.kind !== "found") {
    return (
      <WikiPage header={<PageHeader title="알림" />}>
        <Notice>알림을 읽지 못했습니다.</Notice>
      </WikiPage>
    );
  }

  const { items } = result.data;
  const unread = items.filter((item) => !item.read).length;

  return (
    <WikiPage
      header={
        <PageHeader
          title="알림"
          note={unread > 0 ? `읽지 않은 알림 ${unread}건` : undefined}
          actions={unread > 0 ? <MarkAllReadButton /> : undefined}
        />
      }
    >
      {items.length === 0 ? (
        <Notice>아직 알림이 없습니다.</Notice>
      ) : (
        <Rows>
          {items.map((item, index) => (
            <Row key={`${item.createdAt}:${index}`}>
              {!item.read && (
                <span role="img" aria-label="읽지 않음" className="text-accent">
                  ●
                </span>
              )}
              <span className="text-muted">{item.kindLabel}</span>
              {item.document && (
                <Link
                  href={wikiPath.read(item.document)}
                  className={linkStyle()}
                >
                  {item.document}
                </Link>
              )}
              <span className="ml-auto text-faint">
                {formatMoment(item.createdAt)}
              </span>
            </Row>
          ))}
        </Rows>
      )}
    </WikiPage>
  );
}
