
class MyClass {
    @Override
    protected boolean shouldLog() { return false }

    @Override
    protected void logExtra() { }

    void setUp() {
        dao = new AbstractDao() {
            @Override
            protected boolean shouldLog() { return true }
        }
    }
}
        