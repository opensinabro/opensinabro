"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";

export function Providers({ children }: { children: React.ReactNode }) {
  // 문서 데이터는 서버 컴포넌트가 싣는다. Query가 맡는 것은 편집 미리보기처럼
  // 화면에서 되풀이되는 조회뿐이라, 다시 부르는 조건을 좁게 잡는다.
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { refetchOnWindowFocus: false, retry: 1 },
        },
      }),
  );

  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
