export default function WithHeadChild(props) {
  return (
    <>
      <Head><meta name="child-meta" content="from-child" /></Head>
      <section>{props.label}</section>
    </>
  );
}
