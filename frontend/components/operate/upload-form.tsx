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
import { uploadFile } from "@/lib/api/operate-client";
import type { UploadOptions } from "@/lib/api/operate";
import { wikiPath } from "@/lib/wiki-path";

export function UploadForm({ options }: { options: UploadOptions }) {
  const router = useRouter();
  const [file, setFile] = useState<File | null>(null);
  const [name, setName] = useState("");
  const [license, setLicense] = useState(options.licenses[0]?.name ?? "");
  const [category, setCategory] = useState("");
  const [description, setDescription] = useState("");
  const [uploading, setUploading] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function upload(event: React.FormEvent) {
    event.preventDefault();
    if (!file) return;

    setUploading(true);
    setProblem(null);

    try {
      const uploaded = await uploadFile({
        file,
        name,
        license,
        category,
        description,
      });
      router.push(wikiPath.read(uploaded));
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "파일을 올리지 못했습니다.",
      );
      setUploading(false);
    }
  }

  return (
    <form onSubmit={upload} className="mt-5">
      <FormLayout>
        <Field
          label="파일"
          htmlFor="upload-file"
          hint={`올릴 수 있는 형식: ${options.mediaTypes.join(", ")}`}
        >
          <input
            id="upload-file"
            type="file"
            accept={options.mediaTypes.join(",")}
            onChange={(event) => setFile(event.target.files?.[0] ?? null)}
            required
            className={inputStyle}
          />
        </Field>
        <Field
          label="이름"
          htmlFor="upload-name"
          hint="파일 이름공간의 문서 제목이 됩니다."
        >
          <input
            id="upload-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <Field label="라이선스" htmlFor="upload-license">
          <select
            id="upload-license"
            value={license}
            onChange={(event) => setLicense(event.target.value)}
            required
            className={inputStyle}
          >
            {options.licenses.map((choice) => (
              <option key={choice.name} value={choice.name}>
                {choice.displayName}
              </option>
            ))}
          </select>
        </Field>
        <Field label="분류" htmlFor="upload-category">
          <input
            id="upload-category"
            value={category}
            onChange={(event) => setCategory(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <Field label="설명" htmlFor="upload-description">
          <textarea
            id="upload-description"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            rows={4}
            className={inputStyle}
          />
        </Field>
        <FormActions>
          <button
            type="submit"
            disabled={uploading}
            className={buttonStyle({ tone: "primary" })}
          >
            {uploading ? "올리는 중" : "올리기"}
          </button>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
