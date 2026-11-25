
@SuppressWarnings
class MyClass {
    @Component
    private class MyNestedClass {
        @Provider
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
        