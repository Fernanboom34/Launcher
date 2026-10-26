# Never reopen a verified source by name. FileShare.Read denies writes and deletion
# until the caller has copied from this same handle and disposed it.
function Open-LumaVerifiedFile($path, $expected) {
    $stream = [IO.File]::Open($path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try {
        $hash = [Security.Cryptography.SHA256]::Create()
        try { $actual = [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
        finally { $hash.Dispose() }
        if (!$actual.Equals($expected, [StringComparison]::OrdinalIgnoreCase)) { throw 'Descarga inválida' }
        $stream.Position = 0
        return ,$stream
    } catch { $stream.Dispose(); throw }
}
function Copy-LumaVerifiedStream($source, $destination) {
    $source.Position = 0
    $output = [IO.File]::Open($destination, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $source.CopyTo($output); $output.Flush($true) }
    finally { $output.Dispose() }
}
