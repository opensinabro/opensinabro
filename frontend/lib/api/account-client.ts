"use client";

import { expectOk, postJson } from "./csrf";

// 서버가 내는 실패 사유(계정 없음 / 비밀번호 틀림)를 화면까지 옮기지 않는다 —
// 문구가 갈리면 이름만 넣어 보고 계정의 존재 여부를 알아낼 수 있다.
const loginFailure = "사용자 이름이나 비밀번호가 맞지 않습니다.";

export async function logIn(payload: { name: string; password: string }) {
  const response = await postJson("/api/login", payload);
  if (!response.ok) throw new Error(loginFailure);
}

export async function signUp(payload: {
  name: string;
  email: string;
  password: string;
}) {
  const response = await postJson("/api/signup", payload);
  await expectOk(response, "가입하지 못했습니다.");

  const body = (await response.json()) as { name: string };
  return body.name;
}
