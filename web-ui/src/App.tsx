import { Button } from "@/components/ui/button";

function App() {
  return (
    <main className="min-h-svh bg-background pb-safe pt-safe">
      <div className="container mx-auto px-4 py-8">
        <h1 className="text-2xl font-bold">Tether</h1>
        <p className="text-muted-foreground">Phone proximity tracker</p>
        <div className="mt-4">
          <Button>Click me</Button>
        </div>
      </div>
    </main>
  );
}

export default App;
