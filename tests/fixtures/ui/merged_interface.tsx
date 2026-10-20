interface Props { first: string; }
interface Props { second: number; }

export default function Page(props: Props) {
  return <p>{props.first}{props.second}</p>;
}
