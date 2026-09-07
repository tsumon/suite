using System.IO;
using System.Text.Json;
using Suite.Contracts;

namespace Suite.App;

public sealed class SettingsStore
{
    private readonly object _gate = new();
    private AppSettings _current = AppSettings.CreateDefault();

    public AppSettings Current
    {
        get
        {
            lock (_gate)
            {
                return _current.Clone();
            }
        }
    }

    public string FilePath => SettingsPaths.FilePath;

    public AppSettings Load()
    {
        lock (_gate)
        {
            try
            {
                if (File.Exists(SettingsPaths.FilePath))
                {
                    string json = File.ReadAllText(SettingsPaths.FilePath);
                    int fileSchema = ReadSchemaVersion(json);
                    _current = SettingsJson.Deserialize(json);
                    if (fileSchema < AppSettings.CurrentSchemaVersion)
                    {
                        try
                        {
                            PersistUnlocked(_current);
                        }
                        catch
                        {
                        }
                    }
                }
                else
                {
                    _current = AppSettings.CreateDefault();
                }
            }
            catch
            {
                TryBackupCorruptFile();
                _current = AppSettings.CreateDefault();
            }

            return _current.Clone();
        }
    }

    public void Save(AppSettings settings)
    {
        lock (_gate)
        {
            PersistUnlocked(settings);
        }
    }

    private void PersistUnlocked(AppSettings settings)
    {
        _current = settings.Clone();
        Directory.CreateDirectory(SettingsPaths.DirectoryPath);
        string json = SettingsJson.Serialize(_current);
        string temp = SettingsPaths.FilePath + ".tmp";
        File.WriteAllText(temp, json);
        File.Move(temp, SettingsPaths.FilePath, overwrite: true);
    }

    private static int ReadSchemaVersion(string json)
    {
        try
        {
            using JsonDocument doc = JsonDocument.Parse(json);
            if (doc.RootElement.TryGetProperty("schemaVersion", out JsonElement version)
                && version.TryGetInt32(out int n))
            {
                return n;
            }
        }
        catch
        {
        }

        return 0;
    }

    public void Update(Action<AppSettings> mutate)
    {
        AppSettings next = Current;
        mutate(next);
        Save(next);
    }

    private static void TryBackupCorruptFile()
    {
        try
        {
            if (File.Exists(SettingsPaths.FilePath))
            {
                File.Copy(SettingsPaths.FilePath, SettingsPaths.FilePath + ".bak", overwrite: true);
            }
        }
        catch
        {
        }
    }
}
