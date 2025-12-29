import type { ReactNode } from "react";
import { useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Power, Loader2, AlertCircle, Copy, Check, Link2 } from "lucide-react";
import { getSystemStatus, restartSystem, getDumbpipeTicket } from "@/generated";
import type { SystemStatusResponse, DumbpipeTicketResponse } from "@/generated";

export function SystemSettings(): ReactNode {
  const [showRestartConfirm, setShowRestartConfirm] = useState(false);
  const [ticketCopied, setTicketCopied] = useState(false);

  const statusQuery = useQuery({
    queryKey: ["system", "status"],
    queryFn: async (): Promise<SystemStatusResponse> => {
      const response = await getSystemStatus();
      if (response.error || !response.data) {
        throw new Error("Failed to get system status");
      }
      return response.data;
    },
  });

  const ticketQuery = useQuery({
    queryKey: ["dumbpipe", "ticket"],
    queryFn: async (): Promise<DumbpipeTicketResponse> => {
      const response = await getDumbpipeTicket();
      if (response.error || !response.data) {
        throw new Error("Failed to get dumbpipe ticket");
      }
      return response.data;
    },
  });

  const restartMutation = useMutation({
    mutationFn: async () => {
      const response = await restartSystem({
        body: { delay_secs: 5 },
      });
      if (response.error || !response.data) {
        throw new Error("Failed to restart system");
      }
      return response.data;
    },
  });

  const handleCopyTicket = async () => {
    if (ticketQuery.data?.ticket) {
      try {
        await navigator.clipboard.writeText(ticketQuery.data.ticket);
        setTicketCopied(true);
        setTimeout(() => setTicketCopied(false), 2000);
      } catch {
        // Fallback
        const textArea = document.createElement("textarea");
        textArea.value = ticketQuery.data.ticket;
        document.body.appendChild(textArea);
        textArea.select();
        document.execCommand("copy");
        document.body.removeChild(textArea);
        setTicketCopied(true);
        setTimeout(() => setTicketCopied(false), 2000);
      }
    }
  };

  const formatUptime = (secs: number): string => {
    const days = Math.floor(secs / 86400);
    const hours = Math.floor((secs % 86400) / 3600);
    const minutes = Math.floor((secs % 3600) / 60);
    if (days > 0) {
      return `${days}d ${hours}h ${minutes}m`;
    }
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  if (statusQuery.isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (statusQuery.isError) {
    return (
      <div className="py-8 text-center">
        <AlertCircle className="mx-auto mb-4 h-12 w-12 text-destructive" />
        <p className="text-destructive">Failed to load system information</p>
        <Button variant="outline" className="mt-4" onClick={() => statusQuery.refetch()}>
          Retry
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Device Information</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Version</span>
            <Badge variant="secondary">{statusQuery.data?.version ?? "Unknown"}</Badge>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Uptime</span>
            <span className="text-sm font-medium">
              {statusQuery.data?.uptime_secs ? formatUptime(statusQuery.data.uptime_secs) : "Unknown"}
            </span>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Bluetooth</span>
            <Badge variant={statusQuery.data?.bluetooth_available ? "default" : "secondary"}>
              {statusQuery.data?.bluetooth_available ? "Available" : "Unavailable"}
            </Badge>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Onboarding</span>
            <Badge variant={statusQuery.data?.onboarding_complete ? "default" : "secondary"}>
              {statusQuery.data?.onboarding_complete ? "Complete" : "Pending"}
            </Badge>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <Link2 className="h-4 w-4" />
            Remote Access
          </CardTitle>
        </CardHeader>
        <CardContent>
          {ticketQuery.isLoading ? (
            <div className="flex items-center justify-center py-4">
              <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          ) : ticketQuery.data?.available && ticketQuery.data.ticket ? (
            <div className="space-y-3">
              <div className="rounded-lg bg-muted p-3">
                <code className="break-all font-mono text-xs">
                  {ticketQuery.data.ticket.slice(0, 60)}...
                </code>
              </div>
              <Button variant="outline" size="sm" className="w-full" onClick={handleCopyTicket}>
                {ticketCopied ? (
                  <>
                    <Check className="mr-2 h-4 w-4" />
                    Copied
                  </>
                ) : (
                  <>
                    <Copy className="mr-2 h-4 w-4" />
                    Copy Ticket
                  </>
                )}
              </Button>
              <p className="text-xs text-muted-foreground">
                Use this ticket to connect remotely via the MCP server.
              </p>
            </div>
          ) : (
            <div className="py-4 text-center">
              <p className="text-sm text-muted-foreground">
                {ticketQuery.data?.message || "Remote access not available"}
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      <Separator />

      <Card className="border-destructive/50">
        <CardHeader className="pb-3">
          <CardTitle className="text-base text-destructive">Danger Zone</CardTitle>
        </CardHeader>
        <CardContent>
          <AlertDialog open={showRestartConfirm} onOpenChange={setShowRestartConfirm}>
            <AlertDialogTrigger asChild>
              <Button
                variant="destructive"
                className="w-full"
                disabled={restartMutation.isPending || restartMutation.isSuccess}
              >
                {restartMutation.isPending || restartMutation.isSuccess ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Restarting...
                  </>
                ) : (
                  <>
                    <Power className="mr-2 h-4 w-4" />
                    Restart Device
                  </>
                )}
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Restart Device</AlertDialogTitle>
                <AlertDialogDescription>
                  Are you sure you want to restart the Tether device? This will temporarily disconnect all services and
                  may take a few minutes to complete.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    restartMutation.mutate();
                    setShowRestartConfirm(false);
                  }}
                  className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                >
                  <Power className="mr-2 h-4 w-4" />
                  Restart
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </CardContent>
      </Card>
    </div>
  );
}
