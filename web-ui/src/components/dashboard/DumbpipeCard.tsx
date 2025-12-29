import type { ReactNode } from "react";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Copy, Check, Link2, AlertCircle, ExternalLink } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { cn } from "@/lib/utils";
import { getDumbpipeTicket } from "@/generated";
import type { DumbpipeTicketResponse } from "@/generated";

interface DumbpipeCardProps {
  className?: string;
}

const COPY_FEEDBACK_DURATION = 2000;

export function DumbpipeCard({ className }: DumbpipeCardProps): ReactNode {
  const [isCopied, setIsCopied] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);

  const { data: ticketResponse, isLoading, isError, error, refetch } = useQuery({
    queryKey: ["dumbpipe", "ticket"],
    queryFn: async (): Promise<DumbpipeTicketResponse> => {
      const response = await getDumbpipeTicket();
      if (response.error || !response.data) {
        throw new Error("Failed to fetch dumbpipe ticket");
      }
      return response.data;
    },
    staleTime: 300000,
    retry: 2,
  });

  const handleCopy = async () => {
    if (!ticketResponse?.ticket) return;

    try {
      await navigator.clipboard.writeText(ticketResponse.ticket);
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), COPY_FEEDBACK_DURATION);
    } catch {
      const textArea = document.createElement("textarea");
      textArea.value = ticketResponse.ticket;
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand("copy");
      document.body.removeChild(textArea);
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), COPY_FEEDBACK_DURATION);
    }
  };

  const truncateTicket = (ticket: string, maxLength: number = 80): string => {
    if (ticket.length <= maxLength) return ticket;
    const halfLength = Math.floor((maxLength - 3) / 2);
    return `${ticket.slice(0, halfLength)}...${ticket.slice(-halfLength)}`;
  };

  if (isLoading) {
    return (
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <div className="flex items-center gap-2">
            <Skeleton className="h-5 w-5 rounded" />
            <Skeleton className="h-6 w-36" />
          </div>
          <Skeleton className="mt-1 h-4 w-48" />
        </CardHeader>
        <CardContent className="space-y-4">
          <Skeleton className="h-20 w-full rounded-md" />
          <Skeleton className="h-10 w-full rounded-md" />
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-3/4" />
        </CardContent>
      </Card>
    );
  }

  if (isError) {
    return (
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-lg">
            <Link2 className="h-5 w-5" />
            Remote Access
          </CardTitle>
          <CardDescription>Dumbpipe connection ticket</CardDescription>
        </CardHeader>
        <CardContent>
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error instanceof Error ? error.message : "Failed to load ticket"}</AlertDescription>
          </Alert>
          <Button variant="outline" onClick={() => refetch()} className="mt-4 w-full">
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  const ticket = ticketResponse?.ticket ?? "";
  const isAvailable = ticketResponse?.available ?? false;
  const unavailableMessage = ticketResponse?.message;
  const isLongTicket = ticket.length > 80;
  const displayTicket = isExpanded ? ticket : truncateTicket(ticket);

  if (!isAvailable) {
    return (
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-lg">
            <Link2 className="h-5 w-5" />
            Remote Access
          </CardTitle>
          <CardDescription>Dumbpipe connection ticket</CardDescription>
        </CardHeader>
        <CardContent>
          <Alert>
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{unavailableMessage || "Dumbpipe is not available"}</AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={cn("relative", className)}>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-lg">
          <Link2 className="h-5 w-5" />
          Remote Access
        </CardTitle>
        <CardDescription>Dumbpipe connection ticket for MCP server</CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        <div className="relative">
          <div
            className={cn(
              "max-h-[200px] min-h-[60px] overflow-y-auto break-all rounded-md bg-muted p-3 font-mono text-xs"
            )}
          >
            {displayTicket || <span className="italic text-muted-foreground">No ticket available</span>}
          </div>

          {isLongTicket && (
            <Button
              variant="ghost"
              size="sm"
              className="absolute bottom-1 right-1 h-6 text-xs"
              onClick={() => setIsExpanded(!isExpanded)}
            >
              {isExpanded ? "Show less" : "Show full"}
            </Button>
          )}
        </div>

        <Button variant="outline" className="w-full gap-2" onClick={handleCopy} disabled={!ticket}>
          {isCopied ? (
            <>
              <Check className="h-4 w-4 text-green-500" />
              Copied!
            </>
          ) : (
            <>
              <Copy className="h-4 w-4" />
              Copy Ticket
            </>
          )}
        </Button>

        <div className="space-y-2 text-xs text-muted-foreground">
          <p>
            This ticket allows secure remote access to your Tether device using{" "}
            <a
              href="https://github.com/n0-computer/dumbpipe"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-primary underline-offset-4 hover:underline"
            >
              dumbpipe
              <ExternalLink className="h-3 w-3" />
            </a>
            , a peer-to-peer connection tool powered by{" "}
            <a
              href="https://github.com/n0-computer/iroh"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-primary underline-offset-4 hover:underline"
            >
              iroh
              <ExternalLink className="h-3 w-3" />
            </a>
            .
          </p>
          <p>
            Use this ticket with the MCP server to query your Tether device from anywhere. Set it as the{" "}
            <code className="rounded bg-muted px-1 py-0.5 font-mono">TETHER_TICKET</code> environment variable.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
