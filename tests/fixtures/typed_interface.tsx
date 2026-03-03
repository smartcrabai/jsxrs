interface CardProps {
  title: string;
  count: number;
  active: boolean;
  tags: string[];
}

export default function Card(props: CardProps) {
  return (
    <div>
      <h1>{props.title}</h1>
      <span>{props.count}</span>
    </div>
  );
}
