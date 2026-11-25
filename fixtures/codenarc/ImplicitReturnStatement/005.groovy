
            public class TestClass {
                public InputStream getAsInputStream(byte[] data) {
                    try (ByteArrayOutputStream baos = new ByteArrayOutputStream();
                         ZipOutputStream zos = new ZipOutputStream(baos)) {
                        zos.putNextEntry(new ZipEntry('test.xml'));
                        zos.write(data);
                        zos.closeEntry();
                        return new ByteArrayInputStream(baos.toByteArray());  // ← Explicit return here
                    }
                }
            }
        