
class IndentationTest {
    private static final CacheLoader<String, List<String>> OK = new CacheLoader<String, List<String>>() {
        List<String> load(String key) {
            key.upperCase()
        }
    }
    private static final LoadingCache<String, List<String>> NOT_OK = CacheBuilder.newBuilder().build(
        new CacheLoader<String, List<String>>() {
            List<String> load(String key) {
                key.upperCase()
            }
        }
    )

    void someMethod() {
        CacheBuilder2.newBuilder().build(
            new CacheLoader2<String, List<String>>() {
                List<String> load2(String key) {
                    key.upperCase()
                }
            }
        )
    }

    AstVisitor getAstVisitor() {
        println 'start'
        new ExplicitTypeInstantiationAstVisitor('ArrayList') {
            @Override
            protected String createErrorMessage() {
                'ArrayList objects ..'
            }
        }
    }
}
        