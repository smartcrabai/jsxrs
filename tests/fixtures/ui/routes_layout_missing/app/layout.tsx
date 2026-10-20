interface LayoutProps { locale: string; children: string; }
export default function Layout(props: LayoutProps) { return <html><body>{props.children}</body></html>; }
