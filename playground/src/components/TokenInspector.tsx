import { useState } from 'react'

import { describe } from '@/lib/describe'
import { usePlaygroundStore } from '@/store'

export function TokenInspector() {
  const tokens = usePlaygroundStore((state) => state.tokens)
  const [hovered, setHovered] = useState<number | null>(null)

  const active = hovered !== null ? tokens[hovered] : null
  const meaning = active ? describe(active) : null

  return (
    <div className="flex h-full flex-col">
      <div className="flex min-h-11 items-center gap-2 border-b px-4 py-2 text-sm">
        {active && meaning ? (
          <>
            <span
              className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
              style={{ background: meaning.color }}
            />
            <span className="font-medium">{meaning.label}</span>
            <code className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
              {active.kind} in {active.parent}
            </code>
          </>
        ) : (
          <span className="text-muted-foreground">토큰에 커서를 올리면 의미가 표시됩니다.</span>
        )}
      </div>
      <div className="overflow-auto whitespace-pre-wrap px-4 py-3 font-mono text-sm leading-relaxed">
        {tokens.map((token, index) => {
          const tokenMeaning = describe(token)
          const isHovered = hovered === index
          return (
            <span
              key={index}
              title={tokenMeaning.label}
              onMouseEnter={() => setHovered(index)}
              className={isHovered ? 'rounded-sm underline decoration-dotted' : undefined}
              style={{
                color: tokenMeaning.color,
                background: isHovered ? 'rgba(0,0,0,0.06)' : undefined,
              }}
            >
              {token.text}
            </span>
          )
        })}
      </div>
    </div>
  )
}
