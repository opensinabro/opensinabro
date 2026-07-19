"use client";

import { Link } from "@/components/layout/link";
import { useState } from "react";
import { Alert, Notice } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import {
  Field,
  FormActions,
  FormLayout,
  inputStyle,
} from "@/components/ui/field";
import { signUp } from "@/lib/api/account-client";
import { linkStyle } from "@/components/ui/link";

export function SignupForm() {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [created, setCreated] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setSubmitting(true);
    setProblem(null);

    try {
      setCreated(await signUp({ name, email, password }));
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "가입하지 못했습니다.",
      );
      setSubmitting(false);
    }
  }

  if (created) {
    return (
      <div>
        <Notice>
          {created} 계정을 만들었습니다. 메일로 보낸 확인 링크를 눌러 주세요.
        </Notice>
        <Link href="/login" className={linkStyle({ size: "ui" })}>
          로그인하기
        </Link>
      </div>
    );
  }

  return (
    <form onSubmit={submit}>
      <FormLayout>
        <Field label="사용자 이름" htmlFor="signup-name">
          <input
            id="signup-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
            autoComplete="username"
            className={inputStyle}
          />
        </Field>
        <Field label="이메일" htmlFor="signup-email">
          <input
            id="signup-email"
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            required
            autoComplete="email"
            className={inputStyle}
          />
        </Field>
        <Field
          label="비밀번호"
          htmlFor="signup-password"
          hint="여덟 자 이상으로 지어 주세요."
        >
          <input
            id="signup-password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            required
            minLength={8}
            autoComplete="new-password"
            className={inputStyle}
          />
        </Field>
        <FormActions>
          <button
            type="submit"
            disabled={submitting}
            className={buttonStyle({ tone: "primary" })}
          >
            {submitting ? "만드는 중" : "계정 만들기"}
          </button>
          <Link href="/login" className={linkStyle({ size: "ui" })}>
            이미 계정이 있습니다
          </Link>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
