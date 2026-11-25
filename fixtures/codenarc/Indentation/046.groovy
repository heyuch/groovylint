
project.files(project.configurations.scaconfig.files.findAll { File it -> it.name.endsWith '.aar' }.collect { File it ->
    MessageDigest sha1 = MessageDigest.getInstance('SHA1')
    String inputFile = 'COMMAND=PREPARE_LIBRARY\n' +
        "FILE_PATH=${it.absolutePath}\n"
    String hash = new BigInteger(1, sha1.digest(inputFile.bytes)).toString(16)
    cacheDir + hash + File.separator + 'output/jars/classes.jar'
}).asFileTree
        