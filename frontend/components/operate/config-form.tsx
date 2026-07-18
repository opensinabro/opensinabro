"use client";

import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import {
  Field,
  FormActions,
  FormLayout,
  inputStyle,
} from "@/components/ui/field";
import type { WikiConfiguration } from "@/lib/api/operate";
import { saveConfiguration } from "@/lib/api/operate-client";

export function ConfigForm({ configuration }: { configuration: WikiConfiguration }) {
  const [wikiName, setWikiName] = useState(configuration.wikiName);
  const [mainDocument, setMainDocument] = useState(configuration.mainDocument);
  const [contentLicense, setContentLicense] = useState(
    configuration.contentLicense,
  );
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function save(event: React.FormEvent) {
    event.preventDefault();
    setSaving(true);
    setSaved(false);
    setProblem(null);

    try {
      await saveConfiguration({ wikiName, mainDocument, contentLicense });
      setSaved(true);
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "설정을 저장하지 못했습니다.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <form onSubmit={save} className="mt-5">
      <FormLayout>
        <Field label="위키 이름" htmlFor="config-wiki-name">
          <input
            id="config-wiki-name"
            value={wikiName}
            onChange={(event) => setWikiName(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <Field label="대문 문서" htmlFor="config-main-document">
          <input
            id="config-main-document"
            value={mainDocument}
            onChange={(event) => setMainDocument(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <Field label="본문 라이선스" htmlFor="config-content-license">
          <input
            id="config-content-license"
            value={contentLicense}
            onChange={(event) => setContentLicense(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <FormActions>
          <button
            type="submit"
            disabled={saving}
            className={buttonStyle({ tone: "primary" })}
          >
            {saving ? "저장하는 중" : "저장"}
          </button>
        </FormActions>
      </FormLayout>
      {saved && <Alert>저장했습니다.</Alert>}
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
