import { createContext, useContext, useState, useEffect, useLayoutEffect, useRef, useCallback } from "react";
import { createPortal } from "react-dom";

export interface MenuItemDef {
  label?: string;
  icon?: string;
  shortcut?: string;
  onClick?: () => void;
  danger?: boolean;
  disabled?: boolean;
  divider?: true;
  children?: MenuItemDef[];
}

interface ContextMenuState {
  x: number;
  y: number;
  items: MenuItemDef[];
}

interface ContextMenuCtx {
  show: (x: number, y: number, items: MenuItemDef[]) => void;
  close: () => void;
}

const Ctx = createContext<ContextMenuCtx>({ show: () => {}, close: () => {} });

export function useContextMenu() {
  return useContext(Ctx);
}

export function ContextMenuProvider({ children }: { children: React.ReactNode }) {
  const [menu, setMenu] = useState<ContextMenuState | null>(null);

  const show = useCallback((x: number, y: number, items: MenuItemDef[]) => {
    setMenu({ x, y, items });
  }, []);

  const close = useCallback(() => setMenu(null), []);

  useEffect(() => {
    if (!menu) return;
    const handler = (e: MouseEvent) => {
      // close on any outside click
      const target = e.target as HTMLElement;
      if (!target.closest("[data-context-menu]")) close();
    };
    const keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("mousedown", handler);
    document.addEventListener("keydown", keyHandler);
    return () => {
      document.removeEventListener("mousedown", handler);
      document.removeEventListener("keydown", keyHandler);
    };
  }, [menu, close]);

  return (
    <Ctx.Provider value={{ show, close }}>
      {children}
      {menu && createPortal(
        <MenuRoot x={menu.x} y={menu.y} items={menu.items} onClose={close} />,
        document.body,
      )}
    </Ctx.Provider>
  );
}

function SubMenu({ items, onClose }: { items: MenuItemDef[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [offset, setOffset] = useState(0);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const overflow = rect.bottom - window.innerHeight + 4;
    if (overflow > 0) setOffset(-overflow);
  }, []);

  return (
    <div
      ref={ref}
      data-context-menu="1"
      style={{ position: "absolute", left: "100%", top: `calc(-4px + ${offset}px)`, zIndex: 10000 }}
    >
      <MenuPanel items={items} onClose={onClose} />
    </div>
  );
}

// ─── Menu root ──────────────────────────────────────────────────────────────

function MenuRoot({ x, y, items, onClose }: { x: number; y: number; items: MenuItemDef[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);

  // Clamp to viewport
  const [pos, setPos] = useState({ x, y });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const nx = Math.min(x, window.innerWidth - rect.width - 4);
    const ny = Math.min(y, window.innerHeight - rect.height - 4);
    if (nx !== x || ny !== y) setPos({ x: nx, y: ny });
  }, [x, y]);

  return (
    <div ref={ref} data-context-menu="1" style={{ position: "fixed", left: pos.x, top: pos.y, zIndex: 9999 }}>
      <MenuPanel items={items} onClose={onClose} />
    </div>
  );
}

function MenuPanel({ items, onClose }: { items: MenuItemDef[]; onClose: () => void }) {
  return (
    <div style={{
      background: "var(--paper-2)",
      border: "1px solid var(--rule)",
      borderRadius: "0px",
      boxShadow: "0 8px 32px rgba(0,0,0,0.5)",
      padding: "4px 0",
      minWidth: "160px",
    }}>
      {items.map((item, i) => (
        item.divider
          ? <div key={i} style={{ height: "1px", background: "var(--rule)", margin: "3px 0" }} />
          : <MenuItem key={i} item={item} onClose={onClose} />
      ))}
    </div>
  );
}

function MenuItem({ item, onClose }: { item: MenuItemDef; onClose: () => void }) {
  const [subOpen, setSubOpen] = useState(false);
  const hasChildren = item.children && item.children.length > 0;
  const hoverTimer = useRef<ReturnType<typeof setTimeout>>();

  const handleClick = () => {
    if (item.disabled || hasChildren) return;
    item.onClick?.();
    onClose();
  };

  const handleMouseEnter = () => {
    clearTimeout(hoverTimer.current);
    hoverTimer.current = setTimeout(() => setSubOpen(true), 80);
  };

  const handleMouseLeave = () => {
    clearTimeout(hoverTimer.current);
    hoverTimer.current = setTimeout(() => setSubOpen(false), 150);
  };

  return (
    <div
      data-context-menu="1"
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      style={{
        position: "relative",
        display: "flex",
        alignItems: "center",
        gap: "8px",
        padding: "5px 10px",
        cursor: item.disabled ? "default" : "pointer",
        userSelect: "none",
        color: item.disabled
          ? "var(--ink-4)"
          : item.danger
            ? "var(--red)"
            : "var(--ink)",
        fontSize: "11px",
        fontFamily: "var(--font-mono)",
      }}
      onMouseOver={e => {
        if (!item.disabled) (e.currentTarget as HTMLElement).style.background = "var(--paper-3)";
      }}
      onMouseOut={e => {
        (e.currentTarget as HTMLElement).style.background = "transparent";
      }}
    >
      {item.icon && (
        <span style={{ width: "14px", textAlign: "center", flexShrink: 0, fontSize: "12px" }}>{item.icon}</span>
      )}
      <span style={{ flex: 1 }}>{item.label}</span>
      {item.shortcut && (
        <span style={{ color: "var(--ink-4)", fontSize: "10px" }}>{item.shortcut}</span>
      )}
      {hasChildren && <span style={{ color: "var(--ink-4)", fontSize: "10px" }}>▶</span>}

      {hasChildren && subOpen && (
        <SubMenu items={item.children!} onClose={onClose} />
      )}
    </div>
  );
}
