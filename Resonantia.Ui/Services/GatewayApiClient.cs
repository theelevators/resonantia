using System.Net.Http.Json;
using Resonantia.Ui.Models;

namespace Resonantia.Ui.Services;

public sealed class GatewayApiClient(HttpClient http)
{
    public async Task<HealthResponse?> GetHealthAsync(CancellationToken ct = default)
        => await http.GetFromJsonAsync<HealthResponse>("/health", ct);

    public async Task<ListNodesResponse?> ListNodesAsync(int limit, string? sessionId, CancellationToken ct = default)
    {
        var qs = string.IsNullOrWhiteSpace(sessionId)
            ? $"/api/v1/nodes?limit={limit}"
            : $"/api/v1/nodes?limit={limit}&sessionId={Uri.EscapeDataString(sessionId)}";

        return await http.GetFromJsonAsync<ListNodesResponse>(qs, ct);
    }

    public async Task<GraphResponse?> GetGraphAsync(int limit, string? sessionId, CancellationToken ct = default)
    {
        var qs = string.IsNullOrWhiteSpace(sessionId)
            ? $"/api/v1/graph?limit={limit}"
            : $"/api/v1/graph?limit={limit}&sessionId={Uri.EscapeDataString(sessionId)}";

        return await http.GetFromJsonAsync<GraphResponse>(qs, ct);
    }

    public async Task<StoreContextResponse?> StoreAsync(StoreContextRequest request, CancellationToken ct = default)
    {
        using var response = await http.PostAsJsonAsync("/api/v1/store", request, ct);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadFromJsonAsync<StoreContextResponse>(cancellationToken: ct);
    }

    public async Task<CalibrateSessionResponse?> CalibrateAsync(CalibrateSessionRequest request, CancellationToken ct = default)
    {
        using var response = await http.PostAsJsonAsync("/api/v1/calibrate", request, ct);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadFromJsonAsync<CalibrateSessionResponse>(cancellationToken: ct);
    }
}
