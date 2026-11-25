
class MyClass {
    private void execute() {
        try (def input = url.openStream()
            def output = new ByteArrayOutputStream(4096)) {

            println input.text       
        }
    }

    private void run() {
        try (InputStream input = url.openStream(); OutputStream output = new ByteArrayOutputStream(4096)) {
               println input.text          // wrong column
        }
    }
}
        