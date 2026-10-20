class InitializedAtLoad {
  static value = initialize();
}

function initialize() { return "side effect"; }

export default function Page() { return <main>valid component</main>; }
