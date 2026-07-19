import type { CSSProperties, ReactNode } from "react";
import type { DocumentLinkKind } from "@/lib/namumark/DocumentLinkKind";
import type { ImageLayout } from "@/lib/namumark/ImageLayout";
import type { RenderInline } from "@/lib/namumark/RenderInline";
import type { RenderedFootnote } from "@/lib/namumark/RenderedFootnote";
import type { TableOfContentsEntry } from "@/lib/namumark/TableOfContentsEntry";
import type { TextStyle } from "@/lib/namumark/TextStyle";
import type { VideoProvider } from "@/lib/namumark/VideoProvider";
import { Link } from "@/components/layout/link";
import { encodeTitle } from "@/lib/wiki-path";
import { Blocks, ContainerBlocks } from "./blocks";
import {
  footnoteAnchor,
  insideLink,
  type RenderContext,
  withoutPreviews,
} from "./context";
import { HtmlNodes } from "./html-nodes";
import { classNames, styleObject } from "./style";

export function Inlines({
  inlines,
  context,
}: {
  inlines: RenderInline[];
  context: RenderContext;
}): ReactNode {
  return inlines.map((inline, index) => (
    <Inline key={index} inline={inline} context={context} />
  ));
}

const styleTag: Record<
  TextStyle,
  "strong" | "em" | "del" | "u" | "sup" | "sub"
> = {
  bold: "strong",
  italic: "em",
  strikethrough: "del",
  underline: "u",
  superscript: "sup",
  subscript: "sub",
};

const linkClass: Record<DocumentLinkKind, string> = {
  existing: "wiki-link-internal",
  missing: "wiki-link-internal not-exist",
  current: "wiki-self-link",
};

const embedHost: Record<VideoProvider, string> = {
  youtube: "//www.youtube.com/embed/",
  kakaoTv: "//tv.kakao.com/embed/player/cliplink/",
  nicoVideo: "//embed.nicovideo.jp/watch/",
};

function Inline({
  inline,
  context,
}: {
  inline: RenderInline;
  context: RenderContext;
}): ReactNode {
  switch (inline.type) {
    case "text":
      return inline.text;

    case "lineBreak":
      return <br />;

    case "styled": {
      const Tag = styleTag[inline.style];
      return (
        <Tag>
          <Inlines inlines={inline.content} context={context} />
        </Tag>
      );
    }

    // 여러 줄 리터럴은 블록으로 보인다. 한 줄이면 글 안에 그대로 놓인다.
    case "literal":
      return inline.text.includes("\n") ? (
        <pre>
          <code>{inline.text}</code>
        </pre>
      ) : (
        <code>{inline.text}</code>
      );

    case "colored":
      return (
        <span style={{ color: inline.color.light }}>
          <Inlines inlines={inline.content} context={context} />
        </span>
      );

    case "sized":
      return (
        <span className={sizeClass(inline.level)}>
          <Inlines inlines={inline.content} context={context} />
        </span>
      );

    case "documentLink":
      return (
        <DocumentLink
          className={linkClass[inline.kind]}
          href={documentHref(inline.title, inline.anchor)}
          missing={inline.kind === "missing"}
          title={inline.title}
        >
          <Inlines inlines={inline.display} context={insideLink(context)} />
        </DocumentLink>
      );

    case "externalLink":
      return (
        <ExternalLink
          url={inline.url}
          display={inline.display}
          context={context}
        />
      );

    case "image":
      return (
        <Image
          fileName={inline.fileName}
          url={inline.url}
          layout={inline.layout}
          context={context}
        />
      );

    case "footnoteReference":
      return (
        <FootnoteReference
          label={inline.label}
          number={inline.number}
          tooltip={inline.tooltip}
          context={context}
        />
      );

    case "video":
      return (
        <iframe
          className="wiki-media"
          src={`${embedHost[inline.provider]}${encodeTitle(inline.identifier)}`}
          width={inline.width ?? "640"}
          height={inline.height ?? "360"}
          frameBorder="0"
          allowFullScreen
          loading="lazy"
        />
      );

    case "ruby":
      return (
        <ruby>
          {inline.content}
          <rp>(</rp>
          <rt>
            {inline.color !== null ? (
              <span style={{ color: inline.color }}>{inline.ruby}</span>
            ) : (
              inline.ruby
            )}
          </rt>
          <rp>)</rp>
        </ruby>
      );

    case "math":
      return (
        <span className="wiki-math" data-formula={inline.formula}>
          {`\\(${inline.formula}\\)`}
        </span>
      );

    case "anchor":
      return <a id={inline.name} />;

    case "clearFix":
      return <div className="wiki-clearfix" />;

    case "tableOfContents":
      return <TableOfContents context={context} />;

    case "footnoteSection":
      return <FootnoteSection notes={inline.notes} context={context} />;

    // 감싸는 요소 없이 글 안에 놓이는 블록들.
    case "blocks":
      return <Blocks blocks={inline.blocks} context={context} />;

    case "wikiStyle":
      return (
        <div style={styleObject(inline.style)}>
          <ContainerBlocks blocks={inline.blocks} context={context} />
        </div>
      );

    case "folding":
      return (
        <details className="wiki-folding">
          <summary>{inline.summary === "" ? "More" : inline.summary}</summary>
          <div>
            <ContainerBlocks blocks={inline.blocks} context={context} />
          </div>
        </details>
      );

    case "codeBlock":
      return (
        <pre>
          <code
            className={inline.language !== null ? "hljs" : undefined}
            data-language={inline.language ?? undefined}
          >
            {inline.source}
          </code>
        </pre>
      );

    case "html":
      return <HtmlNodes nodes={inline.nodes} />;

    // 해석하지 못한 매크로는 적힌 그대로 보인다 — 글이 소리 없이 사라지지 않게.
    // 다만 실어 오지 못한 `include`는 지운다: 틀은 남이 쓴 글을 그 자리에 들이는
    // 장치여서, 들이지 못했을 때 남는 호출 표기는 읽는 사람에게 아무 뜻이 없다.
    case "unresolved":
      if (inline.name.toLowerCase() === "include") {
        return null;
      }
      return inline.argument !== null
        ? `[${inline.name}(${inline.argument})]`
        : `[${inline.name}]`;
  }
}

/**
 * 각주 참조. 표기를 누르면 각주 목록으로 가고, 올려 두면 내용을 그 자리에서 보인다.
 *
 * 미리보기는 IR에 있는 평문(`tooltip`)이 아니라 각주 **본문 트리**를 그린다 — 링크와
 * 서식이 살아 있는 채로 보이고, 본문을 떠나지 않고 읽을 수 있다. 여는 동작은 CSS가
 * 맡으므로 이 컴포넌트는 서버에서도 브라우저에서도 똑같이 순수하다.
 */
function FootnoteReference({
  label,
  number,
  tooltip,
  context,
}: {
  label: string;
  number: number;
  tooltip: string;
  context: RenderContext;
}): ReactNode {
  // 라벨이 아니라 참조 번호로 찾는다 — 라벨은 `[각주]` 구간 안에서만 유일하다.
  const note = context.footnoteByReference.get(number);
  const preview = context.footnotePreviews ? note : undefined;
  return (
    <span className="wiki-fn">
      <a
        className="wiki-fn-content"
        // 미리보기를 못 그리는 경우(각주 목록이 아직 안 나온 자리)를 위해 남긴다.
        title={preview === undefined ? tooltip : undefined}
        href={`#${note === undefined ? `fn-${encodeAnchor(label)}` : footnoteAnchor(note)}`}
      >
        {/* 본문 복귀 앵커는 링크 안쪽 빈 span이 갖는다. */}
        <span id={`rfn-${number}`} />[{label}]
      </a>
      {preview !== undefined && (
        <span className="wiki-fn-preview" role="note">
          {/* 안쪽 참조는 평문 툴팁으로 물러난다. 이유는 withoutPreviews 참고. */}
          <Inlines inlines={preview.content} context={withoutPreviews(context)} />
        </span>
      )}
    </span>
  );
}

function ExternalLink({
  url,
  display,
  context,
}: {
  url: string;
  display: RenderInline[] | null;
  context: RenderContext;
}): ReactNode {
  const isJavascript = url.trimStart().toLowerCase().startsWith("javascript:");
  const isImageLink = display?.length === 1 && display[0].type === "image";
  return (
    <a
      className={isImageLink ? "wiki-link-external-image" : "wiki-link-external"}
      href={isJavascript ? "#" : url}
      target="_blank"
      rel="nofollow noopener ugc"
      // 툴팁에는 주소만 싣고 `#` 뒤 조각은 뺀다.
      title={url.split("#")[0]}
    >
      {display !== null ? (
        <Inlines inlines={display} context={insideLink(context)} />
      ) : (
        url
      )}
    </a>
  );
}

/**
 * 이미지는 두 겹의 span으로 감싼다. 바깥이 크기·정렬을 잡고 안쪽이 그 안을 채운다.
 *
 * 채우는 축은 **지정한 축만**이다 — 크기 옵션이 없으면 어느 쪽도 채우지 않는다.
 */
function Image({
  fileName,
  url,
  layout,
  context,
}: {
  fileName: string;
  url: string | null;
  layout: ImageLayout;
  context: RenderContext;
}): ReactNode {
  // 없는 파일은 그 파일 문서로 가는, 없는 문서 링크가 된다.
  if (url === null) {
    const title = `파일:${fileName}`;
    // 이미 링크 안이면 링크를 세우지 않고 글자만 남긴다. `<a>` 안의 `<a>`는 금지다.
    if (context.insideLink) {
      return <span className={linkClass.missing}>{title}</span>;
    }
    return (
      <DocumentLink
        className={linkClass.missing}
        href={documentHref(title, null)}
        missing
        title={title}
      >
        {title}
      </DocumentLink>
    );
  }

  const fillWidth = layout.width !== null;
  const fillHeight = layout.height !== null;
  const outerStyle: CSSProperties = {};
  if (layout.width !== null) outerStyle.width = layout.width;
  if (layout.height !== null) outerStyle.height = layout.height;
  if (layout.backgroundColor !== null) {
    outerStyle.backgroundColor = layout.backgroundColor;
  }

  const innerStyle: CSSProperties = {};
  if (fillHeight) innerStyle.height = "100%";
  if (fillWidth) innerStyle.width = "100%";

  return (
    <span
      className={classNames(
        `wiki-image-align-${layout.align ?? "normal"}`,
        layout.theme !== null && `wiki-theme-${layout.theme}`,
      )}
      style={outerStyle}
    >
      <span
        className="wiki-image-wrapper"
        style={fillWidth || fillHeight ? innerStyle : undefined}
      >
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          height={fillHeight ? "100%" : undefined}
          width={fillWidth ? "100%" : undefined}
          src={url}
          alt={`파일:${fileName}`}
        />
      </span>
    </span>
  );
}

function TableOfContents({ context }: { context: RenderContext }): ReactNode {
  const entries = context.tableOfContents;
  return (
    <div className="wiki-macro-toc" id="toc">
      <details open>
        <summary className="wiki-macro-toc-summary">
          <span className="wiki-chevron" aria-hidden="true" />
          목차
        </summary>
        {entries.length > 0 && (
          <TableOfContentsLevel
            entries={entries}
            depth={entries[0].depth}
            context={context}
          />
        )}
      </details>
    </div>
  );
}

/**
 * 목차 항목을 깊이별 컨테이너로 중첩한다.
 *
 * 하위 항목 묶음은 상위 항목의 **형제**로 오지, 그 안에 들어가지 않는다.
 */
function TableOfContentsLevel({
  entries,
  depth,
  context,
}: {
  entries: readonly TableOfContentsEntry[];
  depth: number;
  context: RenderContext;
}): ReactNode {
  const rendered: ReactNode[] = [];
  let index = 0;
  while (index < entries.length) {
    const entry = entries[index];
    rendered.push(
      <span className="toc-item" key={entry.number}>
        <a href={`#s-${entry.number}`}>{entry.number}</a>
        {". "}
        <Inlines inlines={entry.title} context={context} />
      </span>,
    );
    index += 1;

    const childrenStart = index;
    while (index < entries.length && entries[index].depth > depth) {
      index += 1;
    }
    if (childrenStart < index) {
      rendered.push(
        <TableOfContentsLevel
          key={`${entry.number}-children`}
          entries={entries.slice(childrenStart, index)}
          depth={depth + 1}
          context={context}
        />,
      );
    }
  }
  return <div className="toc-indent">{rendered}</div>;
}

/** `[각주]` 자리. 내용은 트리 최상위 각주 목록이 소유하므로 인덱스로 찾아 그린다. */
function FootnoteSection({
  notes,
  context,
}: {
  notes: number[];
  context: RenderContext;
}): ReactNode {
  const footnotes = notes
    .map((index) => context.footnotes[index])
    .filter((footnote) => footnote !== undefined);
  if (footnotes.length === 0) {
    return null;
  }
  return (
    <div className="wiki-macro-footnote">
      {footnotes.map((footnote) => (
        <Footnote key={footnote.label} note={footnote} context={context} />
      ))}
    </div>
  );
}

function Footnote({
  note,
  context,
}: {
  note: RenderedFootnote;
  context: RenderContext;
}): ReactNode {
  const first = note.referenceNumbers[0];
  return (
    <span className="footnote-list" id={footnoteAnchor(note)}>
      {/* 표기에서 내려온 사람이 읽던 자리로 돌아가는 길. 여러 번 참조된 각주는
          첫 자리로 돌아가고, 나머지 자리는 뒤의 번호들이 따로 가리킨다. */}
      <a className="footnote-back" href={`#rfn-${first}`} title="본문으로">
        ↑
      </a>
      <span>
        {note.referenceNumbers.length === 1 ? (
          <a href={`#${footnoteAnchor(note)}`}>[{note.label}]</a>
        ) : (
          <>
            <a href={`#${footnoteAnchor(note)}`}>[{note.label}]</a>
            {note.referenceNumbers.map((number, index) => (
              <span key={number}>
                {" "}
                <a href={`#rfn-${number}`} title="본문으로">
                  <sup>
                    {first}.{index + 1}
                  </sup>
                </a>
              </span>
            ))}
          </>
        )}{" "}
        <Inlines inlines={note.content} context={context} />
      </span>
    </span>
  );
}

function sizeClass(level: number): string {
  return level >= 0 ? `wiki-size-up-${level}` : `wiki-size-down-${-level}`;
}

function documentHref(title: string, anchor: string | null): string {
  const path = title === "" ? "" : `/w/${encodeTitle(title)}`;
  return anchor !== null ? `${path}#${encodeTitle(anchor)}` : path;
}

/**
 * 본문에서 다른 문서로 가는 링크.
 *
 * 위키에서 가장 잦은 이동이라 여기가 `<a>`이면 문서를 넘길 때마다 브라우저가 셸까지
 * 통째로 다시 세운다 — 헤더와 우측 열이 그대로인데도 흰 화면이 스친다. 셸을 남기고
 * 본문만 갈아 끼우려면 라우터를 타야 한다.
 *
 * 다만 **미리 받아 두지는 않는다**. 문서 한 장은 사용자별 ACL 판정을 거쳐 매번
 * 렌더되고 크기도 메가바이트대라, 화면에 든 링크마다 미리 받으면 문서 하나를 여는
 * 것만으로 수백 건이 서버로 나간다. 링크가 수십 개인 셸과는 사정이 다르다.
 *
 * 제목이 빈 링크(`[[#개요]]`)는 같은 문서 안의 앵커일 뿐이라 라우터가 할 일이 없다.
 */
function DocumentLink({
  title,
  href,
  className,
  missing,
  children,
}: {
  title: string;
  href: string;
  className: string;
  missing: boolean;
  children: ReactNode;
}): ReactNode {
  const shared = {
    className,
    title,
    rel: missing ? "nofollow" : undefined,
  };

  if (title === "") {
    return (
      <a href={href} {...shared}>
        {children}
      </a>
    );
  }

  return (
    <Link href={href} prefetch={false} {...shared}>
      {children}
    </Link>
  );
}

/** 각주 앵커는 문서 경로와 달리 소문자 hex를 쓰고 `:`·`/`도 인코딩한다. */
function encodeAnchor(text: string): string {
  return encodeURIComponent(text)
    .replace(
      /[!'()*]/g,
      (character) => `%${character.charCodeAt(0).toString(16)}`,
    )
    .replace(/%[0-9A-F]{2}/g, (escape) => escape.toLowerCase());
}
