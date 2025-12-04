using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct Buf
{
    public IntPtr ptr;
    public nuint len;
    public nuint cap;
}
