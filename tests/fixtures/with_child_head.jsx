import WithHeadChild from './components/with_head_child';

export default function Page() {
  return (
    <>
      <Head><title>Parent Title</title></Head>
      <div>
        <WithHeadChild label="hello" />
      </div>
    </>
  );
}
