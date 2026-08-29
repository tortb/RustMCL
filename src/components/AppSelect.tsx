import { useLayoutEffect, useRef, useState } from "react";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/react";
import { Check, ChevronDown } from "lucide-react";

/**
 * Apple 风格下拉选择(基于 Headless UI Listbox)。
 * - 触发元素下方展开毛玻璃浮层列表,大圆角,过渡 cubic-bezier(0.32,0.72,0,1)
 * - 当前选中项带 checkmark,hover 高亮
 * - 内置键盘导航(上下键切换 / Enter 选中 / Esc 关闭)
 *
 * options 的 label 由调用方提前拼好(如 "1.21 · Fabric")。
 */
export interface AppSelectOption<V extends string | number> {
  value: V;
  label: string;
}

interface AppSelectProps<V extends string | number> {
  value: V | null;
  onChange: (value: V) => void;
  options: AppSelectOption<V>[];
  /** 未选中时显示的文字,同时作为列表首行(禁用)占位 */
  placeholder?: string;
  disabled?: boolean;
  /** 追加到触发按钮的 Tailwind 类(宽度等) */
  className?: string;
  /** 列表弹出的对齐方向 */
  anchor?: "start" | "end";
}

export function AppSelect<V extends string | number>({
  value,
  onChange,
  options,
  placeholder,
  disabled = false,
  className = "",
  anchor = "start",
}: AppSelectProps<V>) {
  const btnRef = useRef<HTMLButtonElement>(null);
  const [width, setWidth] = useState<number>();

  // 用触发按钮实际宽度对齐浮层,视觉更统一
  useLayoutEffect(() => {
    const el = btnRef.current;
    if (el) setWidth(el.offsetWidth);
  }, []);

  const selected = options.find((o) => o.value === value);
  const empty = value === undefined || value === null || value === "";
  const shown = empty ? placeholder ?? "" : selected?.label ?? "";

  return (
    <Listbox value={value ?? ("" as V)} onChange={onChange} disabled={disabled}>
      <ListboxButton
        ref={btnRef}
        className={`flex items-center justify-between gap-3 rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent focus:ring-2 focus:ring-accent/30 disabled:opacity-50 ${className}`}
      >
        <span className={`truncate text-left ${empty ? "text-ink-3" : ""}`}>{shown}</span>
        <ChevronDown size={16} className="shrink-0 text-ink-3" />
      </ListboxButton>
      <ListboxOptions
        anchor={`bottom ${anchor}`}
        transition
        style={{ width }}
        className="z-50 rounded-[12px] border border-black/[0.06] bg-white/90 p-1 shadow-[0_12px_40px_rgba(0,0,0,0.14)] backdrop-blur-xl outline-none transition duration-150 ease-[cubic-bezier(0.32,0.72,0,1)] data-[closed]:scale-[0.98] data-[closed]:opacity-0 data-[open]:scale-100 data-[open]:opacity-100"
      >
        {placeholder !== undefined && (
          <ListboxOption
            value={"" as V}
            disabled
            className="cursor-default select-none rounded-[8px] px-3 py-2 text-[13px] text-ink-3"
          >
            {placeholder}
          </ListboxOption>
        )}
        {options.map((o) => (
          <ListboxOption
            key={String(o.value)}
            value={o.value}
            className={({ active }) =>
              `flex cursor-pointer select-none items-center justify-between gap-3 rounded-[8px] px-3 py-2 text-[13.5px] transition-colors duration-100 ${
                active ? "bg-black/[0.06] text-ink" : "text-ink-2"
              }`
            }
          >
            {({ selected: isSel }) => (
              <>
                <span className="truncate">{o.label}</span>
                {isSel && <Check size={15} className="shrink-0 text-accent" />}
              </>
            )}
          </ListboxOption>
        ))}
      </ListboxOptions>
    </Listbox>
  );
}
