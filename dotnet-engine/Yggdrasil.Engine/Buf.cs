using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct Buf
{
    public IntPtr ptr;
    public UIntPtr len;
    public UIntPtr cap;
}
