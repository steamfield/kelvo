import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "~/lib/utils";
import { FIELD_LABEL } from "~/widgets/lib/classes";

const Table = React.forwardRef<
  HTMLTableElement,
  React.HTMLAttributes<HTMLTableElement>
>(({ className, ...props }, ref) => (
  <table
    ref={ref}
    className={cn("w-full caption-bottom text-xs", className)}
    {...props}
  />
));
Table.displayName = "Table";

const headerVariants = cva("", {
  variants: { variant: { default: "[&_tr]:border-b", plain: "" } },
  defaultVariants: { variant: "default" },
});
const bodyVariants = cva("", {
  variants: { variant: { default: "[&_tr:last-child]:border-0", plain: "" } },
  defaultVariants: { variant: "default" },
});
const rowVariants = cva("", {
  variants: {
    variant: {
      default:
        "border-border-subtle border-b transition-colors hover:bg-selected data-[state=selected]:bg-selected",
      plain: "border-0 transition-none hover:bg-transparent",
    },
  },
  defaultVariants: { variant: "default" },
});
const headVariants = cva(
  "text-left align-middle [&:has([role=checkbox])]:pr-0",
  {
    variants: { variant: { default: "h-8 px-3", plain: "h-auto px-0" } },
    defaultVariants: { variant: "default" },
  }
);

const TableHeader = React.forwardRef<
  HTMLTableSectionElement,
  React.HTMLAttributes<HTMLTableSectionElement> &
    VariantProps<typeof headerVariants>
>(({ className, variant, ...props }, ref) => (
  <thead
    ref={ref}
    className={cn(headerVariants({ variant }), className)}
    {...props}
  />
));
TableHeader.displayName = "TableHeader";

const TableBody = React.forwardRef<
  HTMLTableSectionElement,
  React.HTMLAttributes<HTMLTableSectionElement> &
    VariantProps<typeof bodyVariants>
>(({ className, variant, ...props }, ref) => (
  <tbody
    ref={ref}
    className={cn(bodyVariants({ variant }), className)}
    {...props}
  />
));
TableBody.displayName = "TableBody";

const TableFooter = React.forwardRef<
  HTMLTableSectionElement,
  React.HTMLAttributes<HTMLTableSectionElement>
>(({ className, ...props }, ref) => (
  <tfoot
    ref={ref}
    className={cn("border-t [&>tr]:last:border-b-0", className)}
    {...props}
  />
));
TableFooter.displayName = "TableFooter";

const TableRow = React.forwardRef<
  HTMLTableRowElement,
  React.HTMLAttributes<HTMLTableRowElement> & VariantProps<typeof rowVariants>
>(({ className, variant, ...props }, ref) => (
  <tr
    ref={ref}
    className={cn(rowVariants({ variant }), className)}
    {...props}
  />
));
TableRow.displayName = "TableRow";

const TableHead = React.forwardRef<
  HTMLTableCellElement,
  React.ThHTMLAttributes<HTMLTableCellElement> &
    VariantProps<typeof headVariants>
>(({ className, variant, ...props }, ref) => (
  <th
    ref={ref}
    className={cn(headVariants({ variant }), FIELD_LABEL, className)}
    {...props}
  />
));
TableHead.displayName = "TableHead";

const TableCell = React.forwardRef<
  HTMLTableCellElement,
  React.TdHTMLAttributes<HTMLTableCellElement>
>(({ className, ...props }, ref) => (
  <td
    ref={ref}
    className={cn(
      "px-3 py-1.5 align-middle [&:has([role=checkbox])]:pr-0",
      className
    )}
    {...props}
  />
));
TableCell.displayName = "TableCell";

const TableCaption = React.forwardRef<
  HTMLTableCaptionElement,
  React.HTMLAttributes<HTMLTableCaptionElement>
>(({ className, ...props }, ref) => (
  <caption
    ref={ref}
    className={cn("mt-4 text-muted-foreground text-xs", className)}
    {...props}
  />
));
TableCaption.displayName = "TableCaption";

export {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableFooter,
  TableHead,
  TableHeader,
  TableRow,
};
