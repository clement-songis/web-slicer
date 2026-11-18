// Génère zip_bomb.3mf (lot 25) : 3MF valide dont 3D/3dmodel.model fait ~1,5 Go
// décompressé (43 M sommets identiques) pour ~1,5 Mo compressé. Sert au test de
// plafond mémoire du worker (engine/tests/worker.rs). Usage :
//   node engine/tests/fixtures/gen_zip_bomb.mjs engine/tests/fixtures/zip_bomb.3mf
import fs from 'node:fs';
import zlib from 'node:zlib';

const out = process.argv[2] ?? 'zip_bomb.3mf';
const VERTICES = 43_000_000;

function entry(name, data, method) {
  return { name: Buffer.from(name), data, method };
}

function deflateStream(chunks) {
  // chunks : générateur de Buffers ; renvoie { compressed, crc, size }
  const d = zlib.createDeflateRaw({ level: 9 });
  const parts = [];
  d.on('data', (b) => parts.push(b));
  let crc = 0, size = 0;
  return new Promise((resolve) => {
    d.on('end', () => resolve({ compressed: Buffer.concat(parts), crc, size }));
    const pump = () => {
      for (;;) {
        const { value, done } = chunks.next();
        if (done) { d.end(); return; }
        crc = zlib.crc32(value, crc); size += value.length;
        if (!d.write(value)) { d.once('drain', pump); return; }
      }
    };
    pump();
  });
}

function* model() {
  yield Buffer.from('<?xml version="1.0" encoding="UTF-8"?>\n<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">\n<resources>\n<object id="1" type="model">\n<mesh>\n<vertices>\n');
  const line = '<vertex x="1.5" y="2.5" z="3.5"/>\n';
  const block = Buffer.from(line.repeat(100_000));
  for (let i = 0; i < VERTICES / 100_000; i++) yield block;
  yield Buffer.from('</vertices>\n<triangles>\n<triangle v1="0" v2="1" v3="2"/>\n</triangles>\n</mesh>\n</object>\n</resources>\n<build>\n<item objectid="1"/>\n</build>\n</model>\n');
}

const types = Buffer.from('<?xml version="1.0" encoding="UTF-8"?>\n<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>\n');
const rels = Buffer.from('<?xml version="1.0" encoding="UTF-8"?>\n<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>\n');

const files = [];
for (const [name, data] of [['[Content_Types].xml', types], ['_rels/.rels', rels]]) {
  const r = await deflateStream([data][Symbol.iterator]());
  files.push({ name: Buffer.from(name), ...r });
}
files.push({ name: Buffer.from('3D/3dmodel.model'), ...(await deflateStream(model())) });

const local = [], central = [];
let offset = 0;
for (const f of files) {
  const h = Buffer.alloc(30);
  h.writeUInt32LE(0x04034b50, 0); h.writeUInt16LE(20, 4); h.writeUInt16LE(0, 6); h.writeUInt16LE(8, 8);
  h.writeUInt32LE(0, 10); h.writeUInt32LE(f.crc >>> 0, 14); h.writeUInt32LE(f.compressed.length, 18);
  h.writeUInt32LE(f.size, 22); h.writeUInt16LE(f.name.length, 26); h.writeUInt16LE(0, 28);
  local.push(h, f.name, f.compressed);
  const c = Buffer.alloc(46);
  c.writeUInt32LE(0x02014b50, 0); c.writeUInt16LE(20, 4); c.writeUInt16LE(20, 6); c.writeUInt16LE(0, 8);
  c.writeUInt16LE(8, 10); c.writeUInt32LE(0, 12); c.writeUInt32LE(f.crc >>> 0, 16);
  c.writeUInt32LE(f.compressed.length, 20); c.writeUInt32LE(f.size, 24); c.writeUInt16LE(f.name.length, 28);
  c.writeUInt32LE(offset, 42);
  central.push(c, f.name);
  offset += 30 + f.name.length + f.compressed.length;
}
const cd = Buffer.concat(central);
const end = Buffer.alloc(22);
end.writeUInt32LE(0x06054b50, 0); end.writeUInt16LE(files.length, 8); end.writeUInt16LE(files.length, 10);
end.writeUInt32LE(cd.length, 12); end.writeUInt32LE(offset, 16);
fs.writeFileSync(out, Buffer.concat([...local, cd, end]));
console.log(out, 'décompressé', files[2].size, 'compressé', files[2].compressed.length);
