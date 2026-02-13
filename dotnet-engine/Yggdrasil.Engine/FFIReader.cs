using System.Runtime.InteropServices;
using System.Text.Json;

namespace Yggdrasil;

internal static class FFIReader
{
    private static JsonSerializerOptions options = new JsonSerializerOptions
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    /// <summary>
    /// Handles response from engine and deserializes value to a complex type (class).
    /// Throws <see cref="YggdrasilEngineException"/> if engine response is an error.
    /// Returns null if engine response is null.
    /// Free's the response pointer.
    /// </summary>
    /// <typeparam name="TRead">The type of object to deserialize to and return</typeparam>
    /// <param name="ptr">Pointer to a string containing the response from FFI. This pointer will be freed</param>
    /// <returns>The result from deserializing the engine response</returns>
    /// <exception cref="YggdrasilEngineException"></exception>
    internal static TRead? ReadComplex<TRead>(IntPtr ptr)
        where TRead : class
    {
        if (ptr == IntPtr.Zero)
        {
            return null;
        }

        var engineResponse = ReadResponse<EngineResponse<TRead>>(ptr);
        if (engineResponse?.StatusCode == "Error")
        {
            throw new YggdrasilEngineException($"Error: {engineResponse?.ErrorMessage}");
        }

        return engineResponse?.Value;
    }

    private static T? ReadResponse<T>(IntPtr ptr)
        where T : class
    {
        if (ptr == IntPtr.Zero)
        {
            throw new YggdrasilEngineException($"Error: unexpected null pointer");
        }

        try
        {
            string? json;

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                json = Marshal.PtrToStringAnsi(ptr);
            else
                json = Marshal.PtrToStringAuto(ptr);

            var result = json != null ? JsonSerializer.Deserialize<T>(json, options) : null;

            return result;
        }
        finally
        {
            FFI.FreeResponse(ptr);
        }
    }
}
