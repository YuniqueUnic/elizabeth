"use client";

import * as React from "react";

import { cn } from "@/lib/utils";

type TooltipContextValue = {
  hidden?: boolean;
};

const TooltipContext = React.createContext<TooltipContextValue>({});

interface TooltipProviderProps {
  children: React.ReactNode;
  delayDuration?: number;
}

function TooltipProvider({ children }: TooltipProviderProps) {
  return <>{children}</>;
}

interface TooltipProps extends React.HTMLAttributes<HTMLSpanElement> {
  hidden?: boolean;
}

function Tooltip({ children, hidden, className, ...props }: TooltipProps) {
  return (
    <TooltipContext.Provider value={{ hidden }}>
      <span
        data-slot="tooltip"
        className={cn("relative inline-flex", className)}
        {...props}
      >
        {children}
      </span>
    </TooltipContext.Provider>
  );
}

interface TooltipTriggerProps extends React.HTMLAttributes<HTMLSpanElement> {
  asChild?: boolean;
}

function TooltipTrigger(
  { asChild, children, className, ...props }: TooltipTriggerProps,
) {
  if (asChild && React.isValidElement(children)) {
    const child = children as React.ReactElement<any>;
    return React.cloneElement(child, {
      ...props,
      className: cn(child.props.className, className),
      "data-slot": "tooltip-trigger",
    });
  }

  return (
    <span data-slot="tooltip-trigger" className={className} {...props}>
      {children}
    </span>
  );
}

interface TooltipContentProps extends React.HTMLAttributes<HTMLDivElement> {
  hidden?: boolean;
  side?: "top" | "right" | "bottom" | "left";
  align?: "start" | "center" | "end";
}

function TooltipContent({
  className,
  children,
  hidden,
  side,
  align,
  style,
  ...props
}: TooltipContentProps) {
  const context = React.useContext(TooltipContext);
  if (hidden ?? context.hidden) {
    return null;
  }

  const positionStyles: React.CSSProperties = (() => {
    switch (side) {
      case "left":
        return { right: "100%", top: "50%", transform: "translateY(-50%)" };
      case "top":
        return { bottom: "100%", left: "50%", transform: "translateX(-50%)" };
      case "bottom":
        return { top: "100%", left: "50%", transform: "translateX(-50%)" };
      case "right":
      default:
        return { left: "100%", top: "50%", transform: "translateY(-50%)" };
    }
  })();

  let alignStyles: React.CSSProperties = {};
  if (side === "left" || side === "right") {
    if (align === "start") {
      alignStyles = { top: "0", transform: "translateY(0)" };
    }
    if (align === "end") {
      alignStyles = { bottom: "0", top: "auto", transform: "translateY(0)" };
    }
  } else {
    if (align === "start") {
      alignStyles = { left: "0", transform: "translateX(0)" };
    }
    if (align === "end") {
      alignStyles = { right: "0", left: "auto", transform: "translateX(0)" };
    }
  }

  return (
    <div
      role="tooltip"
      data-slot="tooltip-content"
      className={cn(
        "pointer-events-none absolute z-50 rounded-md bg-foreground px-3 py-1.5 text-xs text-background shadow-sm",
        className,
      )}
      style={{ ...positionStyles, ...alignStyles, ...style }}
      {...props}
    >
      {children}
    </div>
  );
}

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger };
