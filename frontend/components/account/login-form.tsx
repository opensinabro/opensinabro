"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import {
  Field,
  FormActions,
  FormLayout,
  inputStyle,
} from "@/components/ui/field";
import { logIn } from "@/lib/api/account-client";

export function LoginForm() {
  const router = useRouter();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setSubmitting(true);
    setProblem(null);

    try {
      await logIn({ name, password });
      router.push("/");
      // 셸의 로그인 상태는 서버에서 그려진다 — 다시 받아야 계정 메뉴가 바뀐다.
      router.refresh();
    } catch (error) {
      setProblem(
        error instanceof Error
          ? error.message
          : "사용자 이름이나 비밀번호가 맞지 않습니다.",
      );
      setSubmitting(false);
    }
  }

  return (
    <form onSubmit={submit}>
      <FormLayout>
        <Field label="사용자 이름" htmlFor="login-name">
          <input
            id="login-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
            autoComplete="username"
            className={inputStyle}
          />
        </Field>
        <Field label="비밀번호" htmlFor="login-password">
          <input
            id="login-password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            required
            autoComplete="current-password"
            className={inputStyle}
          />
        </Field>
        <FormActions>
          <button
            type="submit"
            disabled={submitting}
            className={buttonStyle({ tone: "primary" })}
          >
            {submitting ? "확인하는 중" : "로그인"}
          </button>
          <Link href="/signup" className="text-ui text-link hover:underline">
            계정 만들기
          </Link>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
