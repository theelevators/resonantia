using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Resonantia.Web;
using Resonantia.Ui.Services;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<Resonantia.Ui.Components.App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

builder.Services.AddScoped(sp => new HttpClient { BaseAddress = new Uri(builder.HostEnvironment.BaseAddress) });

builder.Services.AddLogging();

builder.Services.Configure<GatewayOptions>(options =>
{
	options.BaseUrl = "http://localhost:5274";
});

builder.Services.AddHttpClient<GatewayApiClient>(client =>
{
	client.BaseAddress = new Uri("http://localhost:5274");
	client.Timeout = TimeSpan.FromSeconds(20);
});

builder.Services.Configure<OllamaOptions>(options =>
{
	options.BaseUrl = "http://localhost:11434";
});

builder.Services.AddHttpClient<OllamaService>(client =>
{
	client.BaseAddress = new Uri("http://localhost:11434");
	client.Timeout = TimeSpan.FromSeconds(120);
});

await builder.Build().RunAsync();
