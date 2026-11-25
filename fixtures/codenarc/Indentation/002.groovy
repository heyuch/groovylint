
class MyClass {
    private class MyNestedClass {
        private void innerMethod() { }
        void execute() {
            def runnable = new Runnable() {
                @Override
                void run() { }
            }
        }
    }
    protected void outerMethod() { }
}
        