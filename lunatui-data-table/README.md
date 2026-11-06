# lunatui-data-table

Data table for lunatui. A [`DataTable`] wraps the core `Table` and `Scrollbar` in a rounded,
bordered frame with a muted bold header, striped rows and an accent cursor row. Rows come from
any `RowSource`, so only the visible rows are read; a scrollbar sits on the right edge when the
rows overflow. Pass a `TableState` for cursor movement, range selection and sorting, and the
table shows the state's sorted view. All colors come from `lunatui-ui-tokens`.
