def run() {
    try {
        println "x"
    } finally {
        throw new RuntimeException("x")
    }
}
