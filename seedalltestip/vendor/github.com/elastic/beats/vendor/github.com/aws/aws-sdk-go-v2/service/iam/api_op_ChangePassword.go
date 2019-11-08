// Code generated by private/model/cli/gen-api/main.go. DO NOT EDIT.

package iam

import (
	"context"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/internal/awsutil"
	"github.com/aws/aws-sdk-go-v2/private/protocol"
	"github.com/aws/aws-sdk-go-v2/private/protocol/query"
)

// Please also see https://docs.aws.amazon.com/goto/WebAPI/iam-2010-05-08/ChangePasswordRequest
type ChangePasswordInput struct {
	_ struct{} `type:"structure"`

	// The new password. The new password must conform to the AWS account's password
	// policy, if one exists.
	//
	// The regex pattern (http://wikipedia.org/wiki/regex) that is used to validate
	// this parameter is a string of characters. That string can include almost
	// any printable ASCII character from the space (\u0020) through the end of
	// the ASCII character range (\u00FF). You can also include the tab (\u0009),
	// line feed (\u000A), and carriage return (\u000D) characters. Any of these
	// characters are valid in a password. However, many tools, such as the AWS
	// Management Console, might restrict the ability to type certain characters
	// because they have special meaning within that tool.
	//
	// NewPassword is a required field
	NewPassword *string `min:"1" type:"string" required:"true"`

	// The IAM user's current password.
	//
	// OldPassword is a required field
	OldPassword *string `min:"1" type:"string" required:"true"`
}

// String returns the string representation
func (s ChangePasswordInput) String() string {
	return awsutil.Prettify(s)
}

// Validate inspects the fields of the type to determine if they are valid.
func (s *ChangePasswordInput) Validate() error {
	invalidParams := aws.ErrInvalidParams{Context: "ChangePasswordInput"}

	if s.NewPassword == nil {
		invalidParams.Add(aws.NewErrParamRequired("NewPassword"))
	}
	if s.NewPassword != nil && len(*s.NewPassword) < 1 {
		invalidParams.Add(aws.NewErrParamMinLen("NewPassword", 1))
	}

	if s.OldPassword == nil {
		invalidParams.Add(aws.NewErrParamRequired("OldPassword"))
	}
	if s.OldPassword != nil && len(*s.OldPassword) < 1 {
		invalidParams.Add(aws.NewErrParamMinLen("OldPassword", 1))
	}

	if invalidParams.Len() > 0 {
		return invalidParams
	}
	return nil
}

// Please also see https://docs.aws.amazon.com/goto/WebAPI/iam-2010-05-08/ChangePasswordOutput
type ChangePasswordOutput struct {
	_ struct{} `type:"structure"`
}

// String returns the string representation
func (s ChangePasswordOutput) String() string {
	return awsutil.Prettify(s)
}

const opChangePassword = "ChangePassword"

// ChangePasswordRequest returns a request value for making API operation for
// AWS Identity and Access Management.
//
// Changes the password of the IAM user who is calling this operation. The AWS
// account root user password is not affected by this operation.
//
// To change the password for a different user, see UpdateLoginProfile. For
// more information about modifying passwords, see Managing Passwords (https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_ManagingLogins.html)
// in the IAM User Guide.
//
//    // Example sending a request using ChangePasswordRequest.
//    req := client.ChangePasswordRequest(params)
//    resp, err := req.Send(context.TODO())
//    if err == nil {
//        fmt.Println(resp)
//    }
//
// Please also see https://docs.aws.amazon.com/goto/WebAPI/iam-2010-05-08/ChangePassword
func (c *Client) ChangePasswordRequest(input *ChangePasswordInput) ChangePasswordRequest {
	op := &aws.Operation{
		Name:       opChangePassword,
		HTTPMethod: "POST",
		HTTPPath:   "/",
	}

	if input == nil {
		input = &ChangePasswordInput{}
	}

	req := c.newRequest(op, input, &ChangePasswordOutput{})
	req.Handlers.Unmarshal.Remove(query.UnmarshalHandler)
	req.Handlers.Unmarshal.PushBackNamed(protocol.UnmarshalDiscardBodyHandler)
	return ChangePasswordRequest{Request: req, Input: input, Copy: c.ChangePasswordRequest}
}

// ChangePasswordRequest is the request type for the
// ChangePassword API operation.
type ChangePasswordRequest struct {
	*aws.Request
	Input *ChangePasswordInput
	Copy  func(*ChangePasswordInput) ChangePasswordRequest
}

// Send marshals and sends the ChangePassword API request.
func (r ChangePasswordRequest) Send(ctx context.Context) (*ChangePasswordResponse, error) {
	r.Request.SetContext(ctx)
	err := r.Request.Send()
	if err != nil {
		return nil, err
	}

	resp := &ChangePasswordResponse{
		ChangePasswordOutput: r.Request.Data.(*ChangePasswordOutput),
		response:             &aws.Response{Request: r.Request},
	}

	return resp, nil
}

// ChangePasswordResponse is the response type for the
// ChangePassword API operation.
type ChangePasswordResponse struct {
	*ChangePasswordOutput

	response *aws.Response
}

// SDKResponseMetdata returns the response metadata for the
// ChangePassword request.
func (r *ChangePasswordResponse) SDKResponseMetdata() *aws.Response {
	return r.response
}
