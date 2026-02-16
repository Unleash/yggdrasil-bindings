using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
internal struct Buf
{
    public IntPtr ptr;
    public nuint len;
    public nuint cap;
}
