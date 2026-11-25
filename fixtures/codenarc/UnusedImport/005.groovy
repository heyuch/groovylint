
            import static com.example.FaultCode.*
            import com.example.Fault

            class MyResourceTest {
                @Test
                void testUpdateUserWidget_UpdateFails() {
                    useStubFormatter(false, [UPDATE_FAILED])
                    def response = resource.getRecords()
                    assert response.status == 404
                }
            }
        