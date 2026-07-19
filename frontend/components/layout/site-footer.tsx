import { Hold } from "@/components/layout/hold";

// 배경 띠는 화면 끝까지 가고 글자는 Hold 안에 선다. 헤더와 같은 규칙이라야 위아래
// 띠의 왼쪽 끝이 한 줄로 맞는다.
export function SiteFooter({ contentLicense }: { contentLicense: string }) {
  return (
    <footer className="mt-auto border-t border-line bg-ground-sub">
      <Hold className="text-fine py-3.5 text-faint">
        <p className="m-0">문서 내용은 {contentLicense}를 따릅니다.</p>
      </Hold>
    </footer>
  );
}
