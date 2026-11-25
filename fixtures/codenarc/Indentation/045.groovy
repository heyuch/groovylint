
class MyClass {
    private void nestedClassUnderCondition() {
        if (true) {
            println 1234
            def runnable = new Runnable() {
                @Override
                void run() {
                    println 6789
                }
            }
        }
    }
}
        