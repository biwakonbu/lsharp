struct SyntheticHttpState {
    table: ResourceTable,
    ctx: WasiCtx,
    next_http_rep: u32,
    outgoing_responses_created: usize,
    response_set_ok: Option<bool>,
}

impl SyntheticHttpState {
    fn new() -> Self {
        let mut builder = WasiCtxBuilder::new();
        Self {
            table: ResourceTable::new(),
            ctx: builder.build(),
            next_http_rep: 1,
            outgoing_responses_created: 0,
            response_set_ok: None,
        }
    }

    fn fresh_resource<T: 'static>(&mut self) -> Resource<T> {
        let rep = self.next_http_rep;
        self.next_http_rep += 1;
        Resource::new_own(rep)
    }

    fn fresh_fields(&mut self) -> Resource<http_types::Fields> {
        self.fresh_resource()
    }

    fn fresh_incoming_body(&mut self) -> Resource<http_types::IncomingBody> {
        self.fresh_resource()
    }

    fn fresh_outgoing_body(&mut self) -> Resource<http_types::OutgoingBody> {
        self.fresh_resource()
    }

    fn fresh_outgoing_request(&mut self) -> Resource<http_types::OutgoingRequest> {
        self.fresh_resource()
    }

    fn fresh_request_options(&mut self) -> Resource<http_types::RequestOptions> {
        self.fresh_resource()
    }

    fn fresh_incoming_response(&mut self) -> Resource<http_types::IncomingResponse> {
        self.fresh_resource()
    }

    fn fresh_outgoing_response(&mut self) -> Resource<http_types::OutgoingResponse> {
        self.fresh_resource()
    }

    fn fresh_future_trailers(&mut self) -> Resource<http_types::FutureTrailers> {
        self.fresh_resource()
    }

    fn fresh_future_incoming_response(&mut self) -> Resource<http_types::FutureIncomingResponse> {
        self.fresh_resource()
    }

    fn fresh_pollable(&mut self) -> Resource<http_types::Pollable> {
        self.fresh_resource()
    }

    fn fresh_input_stream(&mut self) -> Resource<http_types::InputStream> {
        self.fresh_resource()
    }

    fn fresh_output_stream(&mut self) -> Resource<http_types::OutputStream> {
        self.fresh_resource()
    }

    fn is_valid_status_code(status_code: u16) -> bool {
        (100..=599).contains(&status_code)
    }

    fn is_valid_token(name: &str) -> bool {
        !name.is_empty()
            && name.bytes().all(|byte| {
                matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'%' | b'&' | b'\''
                        | b'*' | b'+' | b'-' | b'.' | b'^' | b'_'
                        | b'`' | b'|' | b'~' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
                )
            })
    }

    fn is_valid_scheme(name: &str) -> bool {
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
    }

    fn is_valid_path(path: &str) -> bool {
        path.starts_with('/')
    }

    fn is_valid_authority(authority: &str) -> bool {
        !authority.is_empty() && !authority.chars().any(char::is_whitespace)
    }

    fn is_valid_header_values(values: &[Vec<u8>]) -> bool {
        values
            .iter()
            .all(|value| !value.iter().any(|byte| matches!(byte, b'\r' | b'\n' | 0)))
    }
}

impl WasiView for SyntheticHttpState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl http_types::HostFields for SyntheticHttpState {
    fn new(&mut self) -> Resource<http_types::Fields> {
        self.fresh_fields()
    }

    fn from_list(
        &mut self,
        entries: Vec<(http_types::FieldName, http_types::FieldValue)>,
    ) -> Result<Resource<http_types::Fields>, http_types::HeaderError> {
        if entries.iter().any(|(name, value)| {
            !Self::is_valid_token(name)
                || !Self::is_valid_header_values(std::slice::from_ref(value))
        }) {
            return Err(http_types::HeaderError::InvalidSyntax);
        }
        Ok(self.fresh_fields())
    }

    fn get(
        &mut self,
        _self_: Resource<http_types::Fields>,
        _name: http_types::FieldName,
    ) -> Vec<http_types::FieldValue> {
        Vec::new()
    }

    fn has(&mut self, _self_: Resource<http_types::Fields>, _name: http_types::FieldName) -> bool {
        false
    }

    fn set(
        &mut self,
        _self_: Resource<http_types::Fields>,
        name: http_types::FieldName,
        value: Vec<http_types::FieldValue>,
    ) -> Result<(), http_types::HeaderError> {
        if !Self::is_valid_token(&name) || !Self::is_valid_header_values(&value) {
            return Err(http_types::HeaderError::InvalidSyntax);
        }
        Ok(())
    }

    fn delete(
        &mut self,
        _self_: Resource<http_types::Fields>,
        name: http_types::FieldName,
    ) -> Result<(), http_types::HeaderError> {
        if !Self::is_valid_token(&name) {
            return Err(http_types::HeaderError::InvalidSyntax);
        }
        Ok(())
    }

    fn append(
        &mut self,
        _self_: Resource<http_types::Fields>,
        name: http_types::FieldName,
        value: http_types::FieldValue,
    ) -> Result<(), http_types::HeaderError> {
        if !Self::is_valid_token(&name)
            || !Self::is_valid_header_values(std::slice::from_ref(&value))
        {
            return Err(http_types::HeaderError::InvalidSyntax);
        }
        Ok(())
    }

    fn entries(&mut self, _self_: Resource<http_types::Fields>) -> Vec<(String, Vec<u8>)> {
        Vec::new()
    }

    fn clone(&mut self, _self_: Resource<http_types::Fields>) -> Resource<http_types::Fields> {
        self.fresh_fields()
    }

    fn drop(&mut self, _rep: Resource<http_types::Fields>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostIncomingRequest for SyntheticHttpState {
    fn method(&mut self, _self_: Resource<http_types::IncomingRequest>) -> http_types::Method {
        http_types::Method::Get
    }

    fn path_with_query(&mut self, _self_: Resource<http_types::IncomingRequest>) -> Option<String> {
        Some("/".to_string())
    }

    fn scheme(
        &mut self,
        _self_: Resource<http_types::IncomingRequest>,
    ) -> Option<http_types::Scheme> {
        Some(http_types::Scheme::Https)
    }

    fn authority(&mut self, _self_: Resource<http_types::IncomingRequest>) -> Option<String> {
        Some("example.test".to_string())
    }

    fn headers(
        &mut self,
        _self_: Resource<http_types::IncomingRequest>,
    ) -> Resource<http_types::Headers> {
        self.fresh_fields()
    }

    fn consume(
        &mut self,
        _self_: Resource<http_types::IncomingRequest>,
    ) -> Result<Resource<http_types::IncomingBody>, ()> {
        Ok(self.fresh_incoming_body())
    }

    fn drop(&mut self, _rep: Resource<http_types::IncomingRequest>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostOutgoingRequest for SyntheticHttpState {
    fn new(
        &mut self,
        _headers: Resource<http_types::Headers>,
    ) -> Resource<http_types::OutgoingRequest> {
        self.fresh_outgoing_request()
    }

    fn body(
        &mut self,
        _self_: Resource<http_types::OutgoingRequest>,
    ) -> Result<Resource<http_types::OutgoingBody>, ()> {
        Ok(self.fresh_outgoing_body())
    }

    fn method(&mut self, _self_: Resource<http_types::OutgoingRequest>) -> http_types::Method {
        http_types::Method::Get
    }

    fn set_method(
        &mut self,
        _self_: Resource<http_types::OutgoingRequest>,
        method: http_types::Method,
    ) -> Result<(), ()> {
        if matches!(
            method,
            http_types::Method::Other(ref name) if !Self::is_valid_token(name)
        ) {
            return Err(());
        }
        Ok(())
    }

    fn path_with_query(&mut self, _self_: Resource<http_types::OutgoingRequest>) -> Option<String> {
        Some("/".to_string())
    }

    fn set_path_with_query(
        &mut self,
        _self_: Resource<http_types::OutgoingRequest>,
        path_with_query: Option<String>,
    ) -> Result<(), ()> {
        if path_with_query
            .as_deref()
            .is_some_and(|path| !Self::is_valid_path(path))
        {
            return Err(());
        }
        Ok(())
    }

    fn scheme(
        &mut self,
        _self_: Resource<http_types::OutgoingRequest>,
    ) -> Option<http_types::Scheme> {
        Some(http_types::Scheme::Https)
    }

    fn set_scheme(
        &mut self,
        _self_: Resource<http_types::OutgoingRequest>,
        scheme: Option<http_types::Scheme>,
    ) -> Result<(), ()> {
        if matches!(
            scheme.as_ref(),
            Some(http_types::Scheme::Other(name)) if !Self::is_valid_scheme(name)
        ) {
            return Err(());
        }
        Ok(())
    }

    fn authority(&mut self, _self_: Resource<http_types::OutgoingRequest>) -> Option<String> {
        Some("example.test".to_string())
    }

    fn set_authority(
        &mut self,
        _self_: Resource<http_types::OutgoingRequest>,
        authority: Option<String>,
    ) -> Result<(), ()> {
        if authority
            .as_deref()
            .is_some_and(|value| !Self::is_valid_authority(value))
        {
            return Err(());
        }
        Ok(())
    }

    fn headers(
        &mut self,
        _self_: Resource<http_types::OutgoingRequest>,
    ) -> Resource<http_types::Headers> {
        self.fresh_fields()
    }

    fn drop(&mut self, _rep: Resource<http_types::OutgoingRequest>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostRequestOptions for SyntheticHttpState {
    fn new(&mut self) -> Resource<http_types::RequestOptions> {
        self.fresh_request_options()
    }

    fn connect_timeout(
        &mut self,
        _self_: Resource<http_types::RequestOptions>,
    ) -> Option<http_types::Duration> {
        None
    }

    fn set_connect_timeout(
        &mut self,
        _self_: Resource<http_types::RequestOptions>,
        _duration: Option<http_types::Duration>,
    ) -> Result<(), ()> {
        Ok(())
    }

    fn first_byte_timeout(
        &mut self,
        _self_: Resource<http_types::RequestOptions>,
    ) -> Option<http_types::Duration> {
        None
    }

    fn set_first_byte_timeout(
        &mut self,
        _self_: Resource<http_types::RequestOptions>,
        _duration: Option<http_types::Duration>,
    ) -> Result<(), ()> {
        Ok(())
    }

    fn between_bytes_timeout(
        &mut self,
        _self_: Resource<http_types::RequestOptions>,
    ) -> Option<http_types::Duration> {
        None
    }

    fn set_between_bytes_timeout(
        &mut self,
        _self_: Resource<http_types::RequestOptions>,
        _duration: Option<http_types::Duration>,
    ) -> Result<(), ()> {
        Ok(())
    }

    fn drop(&mut self, _rep: Resource<http_types::RequestOptions>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostResponseOutparam for SyntheticHttpState {
    fn send_informational(
        &mut self,
        _self_: Resource<http_types::ResponseOutparam>,
        status: u16,
        _headers: Resource<http_types::Headers>,
    ) -> Result<(), http_types::ErrorCode> {
        if !(100..=199).contains(&status) {
            return Err(http_types::ErrorCode::HttpProtocolError);
        }
        Ok(())
    }

    fn set(
        &mut self,
        _param: Resource<http_types::ResponseOutparam>,
        response: Result<Resource<http_types::OutgoingResponse>, http_types::ErrorCode>,
    ) {
        self.response_set_ok = Some(response.is_ok());
    }

    fn drop(&mut self, _rep: Resource<http_types::ResponseOutparam>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostIncomingResponse for SyntheticHttpState {
    fn status(&mut self, _self_: Resource<http_types::IncomingResponse>) -> http_types::StatusCode {
        200
    }

    fn headers(
        &mut self,
        _self_: Resource<http_types::IncomingResponse>,
    ) -> Resource<http_types::Headers> {
        self.fresh_fields()
    }

    fn consume(
        &mut self,
        _self_: Resource<http_types::IncomingResponse>,
    ) -> Result<Resource<http_types::IncomingBody>, ()> {
        Ok(self.fresh_incoming_body())
    }

    fn drop(&mut self, _rep: Resource<http_types::IncomingResponse>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostIncomingBody for SyntheticHttpState {
    fn stream(
        &mut self,
        _self_: Resource<http_types::IncomingBody>,
    ) -> Result<Resource<http_types::InputStream>, ()> {
        Ok(self.fresh_input_stream())
    }

    fn finish(
        &mut self,
        _this: Resource<http_types::IncomingBody>,
    ) -> Resource<http_types::FutureTrailers> {
        self.fresh_future_trailers()
    }

    fn drop(&mut self, _rep: Resource<http_types::IncomingBody>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostFutureTrailers for SyntheticHttpState {
    fn subscribe(
        &mut self,
        _self_: Resource<http_types::FutureTrailers>,
    ) -> Resource<http_types::Pollable> {
        self.fresh_pollable()
    }

    fn get(
        &mut self,
        _self_: Resource<http_types::FutureTrailers>,
    ) -> Option<Result<Result<Option<Resource<http_types::Trailers>>, http_types::ErrorCode>, ()>>
    {
        Some(Ok(Ok(None)))
    }

    fn drop(&mut self, _rep: Resource<http_types::FutureTrailers>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostOutgoingResponse for SyntheticHttpState {
    fn new(
        &mut self,
        _headers: Resource<http_types::Headers>,
    ) -> Resource<http_types::OutgoingResponse> {
        self.outgoing_responses_created += 1;
        self.fresh_outgoing_response()
    }

    fn status_code(
        &mut self,
        _self_: Resource<http_types::OutgoingResponse>,
    ) -> http_types::StatusCode {
        200
    }

    fn set_status_code(
        &mut self,
        _self_: Resource<http_types::OutgoingResponse>,
        status_code: http_types::StatusCode,
    ) -> Result<(), ()> {
        if !Self::is_valid_status_code(status_code) {
            return Err(());
        }
        Ok(())
    }

    fn headers(
        &mut self,
        _self_: Resource<http_types::OutgoingResponse>,
    ) -> Resource<http_types::Headers> {
        self.fresh_fields()
    }

    fn body(
        &mut self,
        _self_: Resource<http_types::OutgoingResponse>,
    ) -> Result<Resource<http_types::OutgoingBody>, ()> {
        Ok(self.fresh_outgoing_body())
    }

    fn drop(&mut self, _rep: Resource<http_types::OutgoingResponse>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostOutgoingBody for SyntheticHttpState {
    fn write(
        &mut self,
        _self_: Resource<http_types::OutgoingBody>,
    ) -> Result<Resource<http_types::OutputStream>, ()> {
        Ok(self.fresh_output_stream())
    }

    fn finish(
        &mut self,
        _this: Resource<http_types::OutgoingBody>,
        _trailers: Option<Resource<http_types::Trailers>>,
    ) -> Result<(), http_types::ErrorCode> {
        Ok(())
    }

    fn drop(&mut self, _rep: Resource<http_types::OutgoingBody>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::HostFutureIncomingResponse for SyntheticHttpState {
    fn subscribe(
        &mut self,
        _self_: Resource<http_types::FutureIncomingResponse>,
    ) -> Resource<http_types::Pollable> {
        self.fresh_pollable()
    }

    fn get(
        &mut self,
        _self_: Resource<http_types::FutureIncomingResponse>,
    ) -> Option<Result<Result<Resource<http_types::IncomingResponse>, http_types::ErrorCode>, ()>>
    {
        Some(Ok(Ok(self.fresh_incoming_response())))
    }

    fn drop(&mut self, _rep: Resource<http_types::FutureIncomingResponse>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl http_types::Host for SyntheticHttpState {
    fn http_error_code(
        &mut self,
        _err: Resource<http_types::IoError>,
    ) -> Option<http_types::ErrorCode> {
        None
    }
}

impl outgoing_handler::Host for SyntheticHttpState {
    fn handle(
        &mut self,
        _request: Resource<http_types::OutgoingRequest>,
        _options: Option<Resource<http_types::RequestOptions>>,
    ) -> Result<Resource<http_types::FutureIncomingResponse>, http_types::ErrorCode> {
        Ok(self.fresh_future_incoming_response())
    }
}
