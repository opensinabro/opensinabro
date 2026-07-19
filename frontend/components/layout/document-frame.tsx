import { Link } from "@/components/layout/link";
import { notFound } from "next/navigation";
import {
  DocumentActions,
  type DocumentTab,
} from "@/components/document/document-actions";
import type { TableOfContentsRailEntry } from "@/components/layout/toc-rail";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { linkStyle } from "@/components/ui/link";
import type { Fetched } from "@/lib/api/fetch";

// 문서 라우트는 전부 같은 뼈대다 — 제목·탭 줄을 세우고, 조회의 네 갈래를 갈라, 찾은
// 것만 그린다. 라우트마다 이 분기를 손으로 적으면 갈래를 빠뜨린 화면이 생긴다:
// 실제로 대부분의 화면이 401과 403을 뭉개, 로그인만 하면 되는 사람에게 "권한이
// 없습니다"라고 알렸다. 갈래를 한곳에서 다루면 그 이탈이 구조적으로 불가능해진다.
// 조회 없이 안내만 내는 화면(고를 리비전을 아직 안 골랐을 때 등)도 같은 머리를 써야
// 라우트 사이에서 제목과 탭 줄의 자리가 흔들리지 않는다.
export function DocumentNotice({
  title,
  note,
  tab,
  children,
}: {
  title: string;
  note?: string;
  tab?: DocumentTab;
  children: React.ReactNode;
}) {
  return (
    <WikiPage
      header={
        <PageHeader
          title={title}
          note={note}
          actions={<DocumentActions title={title} current={tab} />}
        />
      }
    >
      {children}
    </WikiPage>
  );
}

export async function DocumentFrame<T>({
  title,
  note,
  noteFor,
  tab,
  result,
  denied,
  allowed,
  variant,
  toolbarFor,
  tocFor,
  children,
}: {
  title: string;
  note?: string;
  /** 찾은 뒤에야 정할 수 있는 보조 설명(예: "r5와 그 직전의 비교"). */
  noteFor?: (data: T) => string;
  /** 지금 켜져 있는 탭. 역링크·원문처럼 탭에 없는 화면은 비운다. */
  tab?: DocumentTab;
  result: Fetched<T>;
  /** 권한이 막혔을 때의 안내. 화면마다 동사가 달라 문장째 받는다. */
  denied: string;
  /** 조회는 됐지만 본문이 "할 수 있는가"를 따로 싣는 화면(이동·삭제)용. */
  allowed?: (data: T) => boolean;
  variant?: "prose" | "full";
  /** 제목 아래 도구 줄. 조회에 성공한 화면만 낸다. */
  toolbarFor?: (data: T) => React.ReactNode;
  /** 우측 축에 세울 문단 목록. 본문이 있는 화면만 낸다. */
  tocFor?: (data: T) => TableOfContentsRailEntry[];
  children: (data: T) => React.ReactNode | Promise<React.ReactNode>;
}) {
  const headerWith = (resolved?: string, toolbar?: React.ReactNode) => (
    <PageHeader
      title={title}
      note={resolved ?? note}
      actions={<DocumentActions title={title} current={tab} />}
      toolbar={toolbar}
    />
  );
  const header = headerWith();

  if (result.kind === "missing") notFound();

  if (result.kind === "unauthorized") {
    return (
      <WikiPage header={header}>
        <Notice>
          로그인해야 볼 수 있습니다.{" "}
          <Link href="/login" className={linkStyle()}>
            로그인하기
          </Link>
        </Notice>
      </WikiPage>
    );
  }

  if (result.kind === "forbidden" || !(allowed?.(result.data) ?? true)) {
    return (
      <WikiPage header={header}>
        <Notice>{denied}</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage
      header={headerWith(noteFor?.(result.data), toolbarFor?.(result.data))}
      variant={variant}
      toc={tocFor?.(result.data)}
    >
      {await children(result.data)}
    </WikiPage>
  );
}
