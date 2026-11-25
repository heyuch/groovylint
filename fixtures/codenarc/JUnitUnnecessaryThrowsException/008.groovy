
        class MyTest {
            @Test
            void shouldDoStuff() throws Exception { }

            @BeforeClass void initialize() throws RuntimeException { }
            @Before void setUp() throws Exception { }
            @After void tearDown() throws Exception { }
            @AfterClass void cleanUp() throws Exception { }
            @Ignore void ignored() throws Exception { }
        }
        