using Suite.Capture.Ocr;
using Windows.Media.Ocr;

Console.WriteLine("AvailableRecognizerLanguages:");
foreach (var lang in OcrEngine.AvailableRecognizerLanguages)
{
    Console.WriteLine("  - " + lang.LanguageTag);
}

var profile = OcrEngine.TryCreateFromUserProfileLanguages();
Console.WriteLine("TryCreateFromUserProfileLanguages: " + (profile is null ? "null" : (profile.RecognizerLanguage?.LanguageTag ?? "?")));

if (WindowsOcrService.TryCreate(out WindowsOcrService? service) && service is not null)
{
    Console.WriteLine("WindowsOcrService.TryCreate: OK lang=" + service.LanguageTag);
    Environment.ExitCode = 0;
}
else
{
    Console.WriteLine("WindowsOcrService.TryCreate: FAIL");
    Environment.ExitCode = 1;
}
