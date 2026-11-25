
class MyClass {
    def doStuff(String name) {
        return {
            def cssInputStream = ClassPathResource.getInputStream(getCssFile())
            assert cssInputStream, "CSS File [$getCssFile()] not found"
            def css = cssInputStream.text
            style(type: 'text/css') {
                unescaped << css
            }
        }
    }
}
        