"use client";

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
import { changePermission } from "@/lib/api/operate-client";

export function GrantForm({ permissions }: { permissions: string[] }) {
  const router = useRouter();
  const [userName, setUserName] = useState("");
  const [permission, setPermission] = useState(permissions[0] ?? "");
  const [revoke, setRevoke] = useState(false);
  const [changing, setChanging] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function change(event: React.FormEvent) {
    event.preventDefault();
    setChanging(true);
    setProblem(null);

    try {
      await changePermission({ userName, permission, revoke });
      router.push("/block-history");
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "권한을 바꾸지 못했습니다.",
      );
      setChanging(false);
    }
  }

  return (
    <form onSubmit={change} className="mt-5">
      <FormLayout>
        <Field label="사용자 이름" htmlFor="grant-user-name">
          <input
            id="grant-user-name"
            value={userName}
            onChange={(event) => setUserName(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <Field label="권한" htmlFor="grant-permission">
          <select
            id="grant-permission"
            value={permission}
            onChange={(event) => setPermission(event.target.value)}
            required
            className={inputStyle}
          >
            {permissions.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </Field>
        <div className="flex items-center gap-2">
          <input
            id="grant-revoke"
            type="checkbox"
            checked={revoke}
            onChange={(event) => setRevoke(event.target.checked)}
          />
          <label htmlFor="grant-revoke" className="text-note text-body">
            권한을 거둡니다
          </label>
        </div>
        <FormActions>
          <button
            type="submit"
            disabled={changing}
            className={buttonStyle({ tone: "primary" })}
          >
            {changing ? "바꾸는 중" : revoke ? "권한 거두기" : "권한 주기"}
          </button>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
