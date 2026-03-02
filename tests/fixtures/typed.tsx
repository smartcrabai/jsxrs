interface Props {
  name: string;
  age: number;
}

export default function Profile(props: Props) {
  return (
    <div>
      <span>{props.name}</span>
      <span>{props.age}</span>
    </div>
  );
}
