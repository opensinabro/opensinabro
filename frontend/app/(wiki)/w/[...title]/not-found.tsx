import {
  MissingDocumentHeader,
  MissingDocumentNotice,
} from "@/components/document/missing-document";
import { WikiPage } from "@/components/layout/wiki-page";

// 없는 문서는 안내 화면을 보이되 상태 코드는 404여야 한다 (docs/architecture.md의 URL 설계).
// 그러려면 페이지가 notFound()로 넘겨야 하고, 그 시점에는 제목이 경로에만 남는다.
export default function DocumentNotFound() {
  return (
    <WikiPage header={<MissingDocumentHeader />}>
      <MissingDocumentNotice />
    </WikiPage>
  );
}
